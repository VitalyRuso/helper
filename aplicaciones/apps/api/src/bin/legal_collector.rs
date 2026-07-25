use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use reqwest::{redirect::Policy, Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const MAX_DOCUMENTS: usize = 20;
const MAX_DOCUMENT_BYTES: usize = 2_000_000;
const MAX_MODEL_CHARS: usize = 80_000;
const USER_AGENT: &str =
    "SpainHelperLegalCollector/1.0 (+https://spain-helper-34-38-50-7.sslip.io)";
const MANIFEST_JSON: &str = r#"
[
  {
    "title": "Ley Organica 4/2000",
    "authority": "BOE",
    "url": "https://www.boe.es/eli/es/lo/2000/01/11/4/con",
    "document_type": "law",
    "legal_area": "immigration",
    "procedure_key": "ley_organica_4_2000"
  },
  {
    "title": "Real Decreto 1155/2024",
    "authority": "BOE",
    "url": "https://www.boe.es/eli/es/rd/2024/11/19/1155/con",
    "document_type": "royal_decree",
    "legal_area": "immigration",
    "procedure_key": "real_decreto_1155_2024"
  },
  {
    "title": "Real Decreto 240/2007",
    "authority": "BOE",
    "url": "https://www.boe.es/eli/es/rd/2007/02/16/240/con",
    "document_type": "royal_decree",
    "legal_area": "immigration",
    "procedure_key": "real_decreto_240_2007"
  },
  {
    "title": "Ley 14/2013",
    "authority": "BOE",
    "url": "https://www.boe.es/eli/es/l/2013/09/27/14/con",
    "document_type": "law",
    "legal_area": "immigration",
    "procedure_key": "ley_14_2013"
  },
  {
    "title": "Real Decreto 1004/2015",
    "authority": "BOE",
    "url": "https://www.boe.es/eli/es/rd/2015/11/06/1004/con",
    "document_type": "royal_decree",
    "legal_area": "nationality",
    "procedure_key": "real_decreto_1004_2015"
  }
]
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = JobConfig::from_env()?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(60))
        .redirect(Policy::limited(5))
        .build()?;
    let manifest: Vec<ManifestItem> = serde_json::from_str(MANIFEST_JSON)?;
    let access_token = vertex_access_token(&client, config.vertex_access_token.as_deref()).await?;

    for item in manifest
        .iter()
        .take(config.max_documents.min(MAX_DOCUMENTS))
    {
        let (final_url, normalized_text) = fetch_official_text(&client, item).await?;
        let analysis = analyze_with_vertex(&client, &config, &access_token, item, &normalized_text)
            .await
            .with_context(|| format!("analyze {}", item.title))?;
        post_intake(
            &client,
            &config,
            item,
            &final_url,
            &normalized_text,
            analysis,
        )
        .await?;
        println!("collected: {} ({})", item.title, final_url);
    }

    Ok(())
}

#[derive(Debug)]
struct JobConfig {
    api_base_url: String,
    collector_token: String,
    project_id: String,
    vertex_location: String,
    vertex_model: String,
    vertex_access_token: Option<String>,
    max_documents: usize,
}

impl JobConfig {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            api_base_url: required("API_BASE_URL")?.trim_end_matches('/').to_owned(),
            collector_token: required("LEGAL_COLLECTOR_TOKEN")?,
            project_id: required("GOOGLE_CLOUD_PROJECT")?,
            vertex_location: std::env::var("VERTEX_LOCATION")
                .unwrap_or_else(|_| "europe-west1".to_owned()),
            vertex_model: std::env::var("VERTEX_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-pro".to_owned()),
            vertex_access_token: std::env::var("VERTEX_ACCESS_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            max_documents: std::env::var("LEGAL_COLLECTOR_MAX_DOCUMENTS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(MAX_DOCUMENTS),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ManifestItem {
    title: String,
    authority: String,
    url: String,
    document_type: String,
    legal_area: String,
    procedure_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelAnalysis {
    summary: String,
    priority: String,
    legal_status: String,
    official_id: Option<String>,
    eli_id: Option<String>,
    sections: Vec<ModelSection>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelSection {
    stable_section_key: String,
    section_type: String,
    section_number: Option<String>,
    title: String,
    text_content: String,
}

async fn fetch_official_text(
    client: &Client,
    item: &ManifestItem,
) -> anyhow::Result<(String, String)> {
    ensure_allowed_url(&item.url)?;
    let response = client.get(&item.url).send().await?.error_for_status()?;
    let final_url = response.url().to_string();
    ensure_allowed_url(&final_url)?;

    if response.content_length().unwrap_or(0) > MAX_DOCUMENT_BYTES as u64 {
        bail!(
            "{} is larger than the configured document limit",
            item.title
        );
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        bail!(
            "{} is larger than the configured document limit",
            item.title
        );
    }
    let text = String::from_utf8_lossy(&bytes);
    let normalized = if content_type.contains("html") {
        html_to_text(&text)?
    } else {
        normalize_text(&text)
    };
    if normalized.len() < 200 {
        bail!("{} produced too little official text", item.title);
    }
    Ok((final_url, normalized))
}

async fn analyze_with_vertex(
    client: &Client,
    config: &JobConfig,
    access_token: &str,
    item: &ManifestItem,
    normalized_text: &str,
) -> anyhow::Result<ModelAnalysis> {
    let model_input = normalized_text
        .chars()
        .take(MAX_MODEL_CHARS)
        .collect::<String>();
    let prompt = format!(
        "Analiza solo el texto oficial recibido. Devuelve JSON estricto. \
         Las secciones deben ser fragmentos exactos del texto, sin inventar hechos, articulos ni citas.\n\
         Documento esperado: {}.\n\n{}",
        item.title, model_input
    );
    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
        config.vertex_location, config.project_id, config.vertex_location, config.vertex_model
    );
    let response: Value = client
        .post(url)
        .bearer_auth(access_token)
        .json(&json!({
            "contents": [{
                "role": "user",
                "parts": [{ "text": prompt }]
            }],
            "generationConfig": {
                "temperature": 0,
                "maxOutputTokens": 4096,
                "responseMimeType": "application/json",
                "responseSchema": response_schema()
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
    item: &ManifestItem,
    final_url: &str,
    normalized_text: &str,
    analysis: ModelAnalysis,
) -> anyhow::Result<()> {
    client
        .post(format!(
            "{}/api/legal-collector/intake",
            config.api_base_url
        ))
        .bearer_auth(&config.collector_token)
        .json(&json!({
            "source": {
                "title": item.authority,
                "authority": item.authority
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
                "model_provider": "vertex",
                "model_name": config.vertex_model,
                "model_location": config.vertex_location,
                "sections": analysis.sections
            }
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
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

fn ensure_allowed_url(value: &str) -> anyhow::Result<()> {
    let url = Url::parse(value)?;
    if url.scheme() != "https" {
        bail!("non-HTTPS URL rejected");
    }
    match url.host_str() {
        Some("boe.es" | "www.boe.es" | "inclusion.gob.es") => Ok(()),
        _ => bail!("non-official URL rejected"),
    }
}

fn response_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "summary": { "type": "STRING" },
            "priority": { "type": "STRING", "enum": ["low", "medium", "high", "critical"] },
            "legal_status": {
                "type": "STRING",
                "enum": ["published", "not_yet_effective", "effective", "partially_effective", "amended", "repealed", "superseded", "pending_consolidation"]
            },
            "official_id": { "type": "STRING", "nullable": true },
            "eli_id": { "type": "STRING", "nullable": true },
            "sections": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "stable_section_key": { "type": "STRING" },
                        "section_type": {
                            "type": "STRING",
                            "enum": ["title", "chapter", "article", "section", "paragraph", "additional_provision", "transitional_provision", "repealing_provision", "final_provision", "annex", "other"]
                        },
                        "section_number": { "type": "STRING", "nullable": true },
                        "title": { "type": "STRING" },
                        "text_content": { "type": "STRING" }
                    },
                    "required": ["stable_section_key", "section_type", "title", "text_content"]
                }
            }
        },
        "required": ["summary", "priority", "legal_status", "sections"]
    })
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} is required"))
}
