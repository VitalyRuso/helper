use std::{collections::BTreeMap, time::Instant};

use anyhow::{anyhow, bail, Context};
use reqwest::{redirect::Policy, Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};

const USER_AGENT: &str =
    "SpainHelperLegalCollector/1.0 (+https://spain-helper-34-38-50-7.sslip.io)";
const MAX_DOCUMENT_BYTES: usize = 2_000_000;
const CHARS_PER_TOKEN: usize = 4;
const FAST_INPUT_USD_PER_1M: f64 = 0.30;
const FAST_OUTPUT_USD_PER_1M: f64 = 2.50;
const REVIEW_INPUT_USD_PER_1M: f64 = 1.25;
const REVIEW_OUTPUT_USD_PER_1M: f64 = 10.00;

const SOURCE_REGISTRY_JSON: &str = r#"
[
  {"id":"boe","authority":"BOE","jurisdiction":"ES","trust_level":"official","allowed_domains":["boe.es","www.boe.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"administracion","authority":"Administracion General del Estado","jurisdiction":"ES","trust_level":"official_secondary","allowed_domains":["administracion.gob.es","www.administracion.gob.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"inclusion","authority":"Ministerio de Inclusion, Seguridad Social y Migraciones","jurisdiction":"ES","trust_level":"official_secondary","allowed_domains":["inclusion.gob.es","www.inclusion.gob.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"interior","authority":"Ministerio del Interior","jurisdiction":"ES","trust_level":"official_secondary","allowed_domains":["interior.gob.es","www.interior.gob.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"exteriores","authority":"Ministerio de Asuntos Exteriores","jurisdiction":"ES","trust_level":"official_secondary","allowed_domains":["exteriores.gob.es","www.exteriores.gob.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"poder_judicial","authority":"Consejo General del Poder Judicial","jurisdiction":"ES","trust_level":"court","allowed_domains":["poderjudicial.es","www.poderjudicial.es"],"ingestion_mode":"html_text","enabled":true},
  {"id":"eur_lex","authority":"EUR-Lex","jurisdiction":"EU","trust_level":"official","allowed_domains":["eur-lex.europa.eu"],"ingestion_mode":"html_text","enabled":true},
  {"id":"curia","authority":"Court of Justice of the European Union","jurisdiction":"EU","trust_level":"court","allowed_domains":["curia.europa.eu"],"ingestion_mode":"html_text","enabled":true}
]
"#;

const SEED_MANIFEST_JSON: &str = r#"
[
  {"source_id":"boe","title":"Ley Organica 4/2000","url":"https://www.boe.es/eli/es/lo/2000/01/11/4/con","document_type":"law","legal_area":"immigration","procedure_key":"ley_organica_4_2000","campaign":"legal_base_v1_extranjeria"},
  {"source_id":"boe","title":"Real Decreto 1155/2024","url":"https://www.boe.es/eli/es/rd/2024/11/19/1155/con","document_type":"royal_decree","legal_area":"immigration","procedure_key":"real_decreto_1155_2024","campaign":"legal_base_v1_extranjeria"},
  {"source_id":"boe","title":"Real Decreto 240/2007","url":"https://www.boe.es/eli/es/rd/2007/02/16/240/con","document_type":"royal_decree","legal_area":"immigration","procedure_key":"real_decreto_240_2007","campaign":"legal_base_v1_familiar_espanol"},
  {"source_id":"inclusion","title":"Hoja 18 - Familiares de personas con nacionalidad espanola","url":"https://www.inclusion.gob.es/web/migraciones/w/18.-autorizacion-de-residencia-temporal-de-familiares-de-personas-con-nacionalidad-espanola","document_type":"info_sheet","legal_area":"immigration","procedure_key":"familiares_nacionalidad_espanola","campaign":"legal_base_v1_familiar_espanol"},
  {"source_id":"inclusion","title":"Modelos generales de extranjeria","url":"https://www.inclusion.gob.es/web/migraciones/modelos-generales","document_type":"form","legal_area":"immigration","procedure_key":"modelos_extranjeria","campaign":"legal_base_v1_extranjeria"},
  {"source_id":"inclusion","title":"Reagrupacion familiar","url":"https://www.inclusion.gob.es/web/migraciones/reagrupacion-familiar-traer-a-sus-familiares-","document_type":"info_sheet","legal_area":"immigration","procedure_key":"reagrupacion_familiar","campaign":"legal_base_v1_extranjeria"}
]
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let config = JobConfig::from_env()?;
    let registry: Vec<TrustedSource> = serde_json::from_str(SOURCE_REGISTRY_JSON)?;
    let manifest: Vec<ManifestItem> = serde_json::from_str(SEED_MANIFEST_JSON)?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(60))
        .redirect(Policy::limited(5))
        .build()?;
    let access_token = if config.dry_run {
        None
    } else {
        Some(vertex_access_token(&client, config.vertex_access_token.as_deref()).await?)
    };
    let mut budget = Budget::new(&config);
    let mut stats = RunStats::default();
    stats.status = "running".to_owned();

    for item in manifest
        .iter()
        .take(config.max_documents_per_run.min(manifest.len()))
    {
        if let Some(status) = budget.stop_status(started) {
            stats.status = status;
            break;
        }
        stats.discovered += 1;
        let source = match trusted_source_for_item(item, &registry) {
            Ok(source) => source,
            Err(error) => {
                stats.failed += 1;
                stats.add_source_failure(&item.source_id);
                stats.errors.push(format!("{}: {error}", item.title));
                continue;
            }
        };

        let result = run_document(
            &client,
            &config,
            access_token.as_deref(),
            source,
            item,
            &mut budget,
            started,
        )
        .await;
        match result {
            Ok(result) => stats.merge(&item.source_id, result),
            Err(error) => {
                let message = error.to_string();
                if message.contains("budget_limit_reached") {
                    stats.status = "budget_limit_reached".to_owned();
                    break;
                }
                if message.contains("limit_reached") {
                    stats.status = "limit_reached".to_owned();
                    break;
                }
                stats.failed += 1;
                stats.add_source_failure(&item.source_id);
                stats.errors.push(format!("{}: {message}", item.title));
            }
        }
    }

    stats.input_tokens = budget.input_tokens;
    stats.output_tokens = budget.output_tokens;
    stats.estimated_cost_usd = round_usd(budget.estimated_cost_usd);
    stats.duration_seconds = started.elapsed().as_secs();
    if stats.status == "running" && manifest.len() > config.max_documents_per_run {
        stats.status = "limit_reached".to_owned();
    }
    if stats.status == "running" {
        stats.status = "completed".to_owned();
    }
    println!("{}", serde_json::to_string_pretty(&stats)?);
    Ok(())
}

async fn run_document(
    client: &Client,
    config: &JobConfig,
    access_token: Option<&str>,
    source: &TrustedSource,
    item: &ManifestItem,
    budget: &mut Budget,
    started: Instant,
) -> anyhow::Result<DocumentRunResult> {
    let mut last_error = None;
    for attempt in 0..=config.max_retries_per_document {
        match run_document_once(client, config, access_token, source, item, budget, started).await {
            Ok(result) => return Ok(result),
            Err(error) => last_error = Some(error),
        }
        if attempt < config.max_retries_per_document {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("document failed")))
}

async fn run_document_once(
    client: &Client,
    config: &JobConfig,
    access_token: Option<&str>,
    source: &TrustedSource,
    item: &ManifestItem,
    budget: &mut Budget,
    started: Instant,
) -> anyhow::Result<DocumentRunResult> {
    let (final_url, normalized_text) = timeout(
        budget.remaining_duration(started),
        fetch_official_text(client, item, source),
    )
    .await
    .context("document timeout")??;

    let class_prompt = classification_prompt(item, &normalized_text);
    let class = if config.dry_run {
        budget.reserve(ModelTier::Fast, estimate_tokens(&class_prompt), 512)?;
        Classification::default_for(item)
    } else {
        model_json::<Classification>(
            client,
            config,
            access_token.context("missing access token")?,
            &config.vertex_fast_model,
            ModelTier::Fast,
            budget,
            class_prompt,
            512,
        )
        .await?
    };
    if !class.relevant {
        return Ok(DocumentRunResult {
            downloaded: 1,
            ..Default::default()
        });
    }

    let extraction_tier = if source.trust_level == "court"
        || class.complexity == "complex"
        || class.confidence < 0.70
    {
        ModelTier::Review
    } else {
        ModelTier::Fast
    };
    let extraction_model = match extraction_tier {
        ModelTier::Fast => &config.vertex_fast_model,
        ModelTier::Review => &config.vertex_review_model,
    };
    let extract_prompt = extraction_prompt(item, &normalized_text);
    let mut analysis = if config.dry_run {
        budget.reserve(
            extraction_tier,
            estimate_tokens(&extract_prompt),
            config.max_output_tokens_per_request as u64,
        )?;
        ModelAnalysis::deterministic(item, &normalized_text)
    } else {
        model_json::<ModelAnalysis>(
            client,
            config,
            access_token.unwrap(),
            extraction_model,
            extraction_tier,
            budget,
            extract_prompt,
            config.max_output_tokens_per_request,
        )
        .await?
    };
    analysis.model_provider = Some("vertex".to_owned());
    analysis.model_name = Some(extraction_model.to_owned());
    analysis.model_tier = Some(extraction_tier.as_str().to_owned());
    analysis.final_review_model = Some(config.vertex_review_model.clone());

    let review_prompt = final_review_prompt(item, &normalized_text, &analysis)?;
    if config.dry_run {
        budget.reserve(ModelTier::Review, estimate_tokens(&review_prompt), 512)?;
    } else {
        let review: FinalReview = model_json(
            client,
            config,
            access_token.unwrap(),
            &config.vertex_review_model,
            ModelTier::Review,
            budget,
            review_prompt,
            512,
        )
        .await?;
        if !review.ok {
            bail!("final review rejected: {}", review.note);
        }
    }

    if config.dry_run {
        return Ok(DocumentRunResult {
            downloaded: 1,
            sections_created: analysis.sections.len() as u64,
            ..Default::default()
        });
    }

    let intake = post_intake(
        client,
        config,
        source,
        item,
        &final_url,
        &normalized_text,
        analysis,
    )
    .await?;
    Ok(DocumentRunResult {
        downloaded: 1,
        unchanged: u64::from(intake.status == "unchanged"),
        versions_created: intake.versions_created as u64,
        sections_created: intake.sections_created as u64,
        review_tasks_created: intake.review_tasks_created as u64,
    })
}

#[derive(Debug)]
struct JobConfig {
    api_base_url: String,
    collector_token: String,
    project_id: String,
    vertex_location: String,
    vertex_fast_model: String,
    vertex_review_model: String,
    vertex_access_token: Option<String>,
    dry_run: bool,
    max_documents_per_run: usize,
    max_model_requests_per_run: u64,
    max_input_tokens_per_run: u64,
    max_output_tokens_per_run: u64,
    max_estimated_cost_usd: f64,
    max_job_duration_seconds: u64,
    max_retries_per_document: u32,
    max_output_tokens_per_request: u32,
}

impl JobConfig {
    fn from_env() -> anyhow::Result<Self> {
        let smoke = bool_env("LEGAL_COLLECTOR_SMOKE_RUN", false);
        Ok(Self {
            api_base_url: required("API_BASE_URL")?.trim_end_matches('/').to_owned(),
            collector_token: required("LEGAL_COLLECTOR_TOKEN")?,
            project_id: required("GOOGLE_CLOUD_PROJECT")?,
            vertex_location: env_string("VERTEX_LOCATION", "europe-west1"),
            vertex_fast_model: env_string("VERTEX_FAST_MODEL", "gemini-2.5-flash"),
            vertex_review_model: env_string("VERTEX_REVIEW_MODEL", "gemini-2.5-pro"),
            vertex_access_token: std::env::var("VERTEX_ACCESS_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            dry_run: bool_env("LEGAL_COLLECTOR_DRY_RUN", false),
            max_documents_per_run: env_parse("MAX_DOCUMENTS_PER_RUN", if smoke { 5 } else { 20 })?,
            max_model_requests_per_run: env_parse("MAX_MODEL_REQUESTS_PER_RUN", 60)?,
            max_input_tokens_per_run: env_parse("MAX_INPUT_TOKENS_PER_RUN", 150_000)?,
            max_output_tokens_per_run: env_parse("MAX_OUTPUT_TOKENS_PER_RUN", 40_000)?,
            max_estimated_cost_usd: env_parse("MAX_ESTIMATED_COST_USD", 10.0)?,
            max_job_duration_seconds: env_parse("MAX_JOB_DURATION_SECONDS", 900)?,
            max_retries_per_document: env_parse("MAX_RETRIES_PER_DOCUMENT", 0)?,
            max_output_tokens_per_request: env_parse("MAX_OUTPUT_TOKENS_PER_REQUEST", 4096)?,
        })
    }
}

#[derive(Debug)]
struct Budget {
    model_requests: u64,
    input_tokens: u64,
    output_tokens: u64,
    estimated_cost_usd: f64,
    max_model_requests: u64,
    max_input_tokens: u64,
    max_output_tokens: u64,
    max_estimated_cost_usd: f64,
    max_duration: Duration,
}

impl Budget {
    fn new(config: &JobConfig) -> Self {
        Self {
            model_requests: 0,
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
            max_model_requests: config.max_model_requests_per_run,
            max_input_tokens: config.max_input_tokens_per_run,
            max_output_tokens: config.max_output_tokens_per_run,
            max_estimated_cost_usd: config.max_estimated_cost_usd,
            max_duration: Duration::from_secs(config.max_job_duration_seconds),
        }
    }

    fn reserve(
        &mut self,
        tier: ModelTier,
        input_tokens: u64,
        output_tokens: u64,
    ) -> anyhow::Result<()> {
        let estimated = model_cost(tier, input_tokens, output_tokens);
        if self.model_requests + 1 > self.max_model_requests
            || self.input_tokens + input_tokens > self.max_input_tokens
            || self.output_tokens + output_tokens > self.max_output_tokens
        {
            bail!("limit_reached");
        }
        if self.estimated_cost_usd + estimated > self.max_estimated_cost_usd {
            bail!("budget_limit_reached");
        }
        self.model_requests += 1;
        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens;
        self.estimated_cost_usd += estimated;
        Ok(())
    }

    fn stop_status(&self, started: Instant) -> Option<String> {
        if started.elapsed() >= self.max_duration {
            Some("limit_reached".to_owned())
        } else if self.estimated_cost_usd >= self.max_estimated_cost_usd {
            Some("budget_limit_reached".to_owned())
        } else if self.model_requests >= self.max_model_requests
            || self.input_tokens >= self.max_input_tokens
            || self.output_tokens >= self.max_output_tokens
        {
            Some("limit_reached".to_owned())
        } else {
            None
        }
    }

    fn remaining_duration(&self, started: Instant) -> Duration {
        self.max_duration.saturating_sub(started.elapsed())
    }
}

#[derive(Debug, Copy, Clone)]
enum ModelTier {
    Fast,
    Review,
}

impl ModelTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Review => "review",
        }
    }
}

#[derive(Debug, Deserialize)]
struct TrustedSource {
    id: String,
    authority: String,
    jurisdiction: String,
    trust_level: String,
    allowed_domains: Vec<String>,
    ingestion_mode: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestItem {
    source_id: String,
    title: String,
    url: String,
    document_type: String,
    legal_area: String,
    procedure_key: String,
    campaign: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Classification {
    relevant: bool,
    complexity: String,
    confidence: f32,
    reason: String,
}

impl Classification {
    fn default_for(item: &ManifestItem) -> Self {
        Self {
            relevant: true,
            complexity: if item.document_type == "court_decision" {
                "complex".to_owned()
            } else {
                "simple".to_owned()
            },
            confidence: 1.0,
            reason: "dry-run deterministic classification".to_owned(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelAnalysis {
    summary: String,
    priority: String,
    legal_status: String,
    confidence: f32,
    official_id: Option<String>,
    eli_id: Option<String>,
    sections: Vec<ModelSection>,
    #[serde(default)]
    model_provider: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    model_tier: Option<String>,
    #[serde(default)]
    final_review_model: Option<String>,
}

impl ModelAnalysis {
    fn deterministic(item: &ManifestItem, text: &str) -> Self {
        let excerpt = text.chars().take(20_000).collect::<String>();
        Self {
            summary: format!("Dry-run extraction for {}", item.title),
            priority: "medium".to_owned(),
            legal_status: "effective".to_owned(),
            confidence: 1.0,
            official_id: None,
            eli_id: None,
            sections: vec![ModelSection {
                stable_section_key: "document-excerpt".to_owned(),
                section_type: "other".to_owned(),
                section_number: None,
                title: item.title.clone(),
                text_content: excerpt,
            }],
            model_provider: Some("dry_run".to_owned()),
            model_name: Some("deterministic".to_owned()),
            model_tier: Some("none".to_owned()),
            final_review_model: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelSection {
    stable_section_key: String,
    section_type: String,
    section_number: Option<String>,
    title: String,
    text_content: String,
}

#[derive(Debug, Deserialize)]
struct FinalReview {
    ok: bool,
    note: String,
}

#[derive(Debug, Deserialize)]
struct IntakeResponse {
    status: String,
    versions_created: u32,
    sections_created: u32,
    review_tasks_created: u32,
}

#[derive(Debug, Default, Serialize)]
struct RunStats {
    status: String,
    discovered: u64,
    downloaded: u64,
    unchanged: u64,
    versions_created: u64,
    sections_created: u64,
    review_tasks_created: u64,
    failed: u64,
    input_tokens: u64,
    output_tokens: u64,
    estimated_cost_usd: f64,
    duration_seconds: u64,
    per_source: BTreeMap<String, SourceStats>,
    errors: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct SourceStats {
    discovered: u64,
    downloaded: u64,
    unchanged: u64,
    versions_created: u64,
    sections_created: u64,
    review_tasks_created: u64,
    failed: u64,
}

#[derive(Debug, Default)]
struct DocumentRunResult {
    downloaded: u64,
    unchanged: u64,
    versions_created: u64,
    sections_created: u64,
    review_tasks_created: u64,
}

impl RunStats {
    fn merge(&mut self, source_id: &str, result: DocumentRunResult) {
        self.downloaded += result.downloaded;
        self.unchanged += result.unchanged;
        self.versions_created += result.versions_created;
        self.sections_created += result.sections_created;
        self.review_tasks_created += result.review_tasks_created;
        let source = self.per_source.entry(source_id.to_owned()).or_default();
        source.discovered += 1;
        source.downloaded += result.downloaded;
        source.unchanged += result.unchanged;
        source.versions_created += result.versions_created;
        source.sections_created += result.sections_created;
        source.review_tasks_created += result.review_tasks_created;
    }

    fn add_source_failure(&mut self, source_id: &str) {
        let source = self.per_source.entry(source_id.to_owned()).or_default();
        source.discovered += 1;
        source.failed += 1;
    }
}

async fn fetch_official_text(
    client: &Client,
    item: &ManifestItem,
    source: &TrustedSource,
) -> anyhow::Result<(String, String)> {
    ensure_allowed_url(&item.url, source)?;
    let response = client.get(&item.url).send().await?.error_for_status()?;
    let final_url = response.url().to_string();
    ensure_allowed_url(&final_url, source)?;
    if response.content_length().unwrap_or(0) > MAX_DOCUMENT_BYTES as u64 {
        bail!("document is larger than configured limit");
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        bail!("document is larger than configured limit");
    }
    let text = String::from_utf8_lossy(&bytes);
    let normalized = if content_type.contains("html") {
        html_to_text(&text)?
    } else {
        normalize_text(&text)
    };
    if normalized.len() < 200 {
        bail!("too little official text extracted");
    }
    Ok((final_url, normalized))
}

async fn model_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    config: &JobConfig,
    access_token: &str,
    model: &str,
    tier: ModelTier,
    budget: &mut Budget,
    prompt: String,
    max_output_tokens: u32,
) -> anyhow::Result<T> {
    let input_tokens = estimate_tokens(&prompt);
    budget.reserve(tier, input_tokens, max_output_tokens as u64)?;
    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
        config.vertex_location, config.project_id, config.vertex_location, model
    );
    let response: Value = client
        .post(url)
        .bearer_auth(access_token)
        .json(&json!({
            "contents": [{ "role": "user", "parts": [{ "text": prompt }] }],
            "generationConfig": {
                "temperature": 0,
                "maxOutputTokens": max_output_tokens,
                "responseMimeType": "application/json"
            }
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let text = response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Vertex response did not include JSON text"))?;
    Ok(serde_json::from_str(text)?)
}

async fn post_intake(
    client: &Client,
    config: &JobConfig,
    source: &TrustedSource,
    item: &ManifestItem,
    final_url: &str,
    normalized_text: &str,
    analysis: ModelAnalysis,
) -> anyhow::Result<IntakeResponse> {
    Ok(client
        .post(format!(
            "{}/api/legal-collector/intake",
            config.api_base_url
        ))
        .bearer_auth(&config.collector_token)
        .json(&json!({
            "source": {
                "registry_id": source.id,
                "title": source.authority,
                "authority": source.authority,
                "jurisdiction": source.jurisdiction,
                "trust_level": source.trust_level,
                "ingestion_mode": source.ingestion_mode
            },
            "document": {
                "official_id": analysis.official_id,
                "eli_id": analysis.eli_id,
                "title": item.title,
                "document_type": item.document_type,
                "legal_area": item.legal_area,
                "procedure_key": item.procedure_key,
                "source_url": final_url,
                "status": "active"
            },
            "version": {
                "version_label": "collector",
                "normalized_text": normalized_text,
                "legal_status": analysis.legal_status
            },
            "analysis": {
                "summary": analysis.summary,
                "priority": analysis.priority,
                "change_type": "modified_wording",
                "model_provider": analysis.model_provider,
                "model_name": analysis.model_name,
                "model_location": config.vertex_location,
                "sections": analysis.sections
            }
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn vertex_access_token(client: &Client, explicit: Option<&str>) -> anyhow::Result<String> {
    if let Some(token) = explicit {
        return Ok(token.to_owned());
    }
    let response: Value = client
        .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
        .header("Metadata-Flavor", "Google")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    response["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("metadata server did not return an access token"))
}

fn classification_prompt(item: &ManifestItem, text: &str) -> String {
    format!(
        "Return JSON only: {{\"relevant\":bool,\"complexity\":\"simple|complex\",\"confidence\":0..1,\"reason\":\"short\"}}. \
         Decide relevance for Spain Helper extranjeria/familiar de ciudadano espanol. Title: {} Campaign: {} Text:\n{}",
        item.title,
        item.campaign,
        text.chars().take(30_000).collect::<String>()
    )
}

fn extraction_prompt(item: &ManifestItem, text: &str) -> String {
    format!(
        "Return JSON only with fields summary, priority, legal_status, confidence, official_id, eli_id, sections. \
         Sections must be exact excerpts copied from the provided official Spanish text. Do not invent facts. \
         Title: {}\nText:\n{}",
        item.title,
        text.chars().take(80_000).collect::<String>()
    )
}

fn final_review_prompt(
    item: &ManifestItem,
    text: &str,
    analysis: &ModelAnalysis,
) -> anyhow::Result<String> {
    Ok(format!(
        "Return JSON only: {{\"ok\":bool,\"note\":\"short\"}}. Verify that every section.text_content exists verbatim in the official text and the analysis does not invent facts. Title: {}\nAnalysis:\n{}\nOfficial text:\n{}",
        item.title,
        serde_json::to_string(analysis)?,
        text.chars().take(40_000).collect::<String>()
    ))
}

fn trusted_source_for_item<'a>(
    item: &ManifestItem,
    registry: &'a [TrustedSource],
) -> anyhow::Result<&'a TrustedSource> {
    let source = registry
        .iter()
        .find(|source| source.id == item.source_id && source.enabled)
        .ok_or_else(|| anyhow!("trusted source is disabled or missing"))?;
    ensure_allowed_url(&item.url, source)?;
    Ok(source)
}

fn ensure_allowed_url(value: &str, source: &TrustedSource) -> anyhow::Result<()> {
    let url = Url::parse(value)?;
    if url.scheme() != "https" {
        bail!("non-HTTPS URL rejected");
    }
    let host = url.host_str().ok_or_else(|| anyhow!("URL has no host"))?;
    if source.allowed_domains.iter().any(|domain| domain == host) {
        Ok(())
    } else {
        bail!("URL host is outside trusted source registry")
    }
}

fn html_to_text(html: &str) -> anyhow::Result<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("body").map_err(|_| anyhow!("invalid selector"))?;
    let text = document
        .select(&selector)
        .flat_map(|body| body.text())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(normalize_text(&text))
}

fn normalize_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn estimate_tokens(text: &str) -> u64 {
    text.len().div_ceil(CHARS_PER_TOKEN) as u64
}

fn model_cost(tier: ModelTier, input_tokens: u64, output_tokens: u64) -> f64 {
    let (input_rate, output_rate) = match tier {
        ModelTier::Fast => (FAST_INPUT_USD_PER_1M, FAST_OUTPUT_USD_PER_1M),
        ModelTier::Review => (REVIEW_INPUT_USD_PER_1M, REVIEW_OUTPUT_USD_PER_1M),
    };
    (input_tokens as f64 * input_rate + output_tokens as f64 * output_rate) / 1_000_000.0
}

fn round_usd(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} is required"))
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn env_parse<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.parse().with_context(|| format!("{key} is invalid")))
        .unwrap_or(Ok(default))
}

fn bool_env(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_accepts_only_declared_domains() {
        let source: TrustedSource = serde_json::from_value(json!({
            "id": "boe",
            "authority": "BOE",
            "jurisdiction": "ES",
            "trust_level": "official",
            "allowed_domains": ["www.boe.es"],
            "ingestion_mode": "html_text",
            "enabled": true
        }))
        .unwrap();
        assert!(
            ensure_allowed_url("https://www.boe.es/eli/es/lo/2000/01/11/4/con", &source).is_ok()
        );
        assert!(ensure_allowed_url("https://example.com", &source).is_err());
    }

    #[test]
    fn budget_rejects_cost_over_limit() {
        let mut budget = Budget {
            model_requests: 0,
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
            max_model_requests: 10,
            max_input_tokens: 1_000_000,
            max_output_tokens: 1_000_000,
            max_estimated_cost_usd: 0.01,
            max_duration: Duration::from_secs(10),
        };
        assert!(budget
            .reserve(ModelTier::Review, 1_000_000, 1_000_000)
            .is_err());
    }
}
