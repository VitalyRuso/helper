use std::future::Future;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::{models::KnowledgeItemView, repository};

const MAX_CONTEXT_ITEMS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerLanguage {
    Es,
    Ru,
    En,
}

impl AnswerLanguage {
    fn code(self) -> &'static str {
        match self {
            Self::Es => "es",
            Self::Ru => "ru",
            Self::En => "en",
        }
    }

    fn prompt_name(self) -> &'static str {
        match self {
            Self::Es => "Spanish",
            Self::Ru => "Russian",
            Self::En => "English",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionRoute {
    General,
    Legal(LegalRoute),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalRoute {
    pub language: AnswerLanguage,
    matched_rules: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalAnswer {
    pub answer: String,
    pub metadata: LegalMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalMetadata {
    pub language: String,
    pub legal_area: Option<String>,
    pub procedure_key: Option<String>,
    pub reviewed: bool,
    pub reviewer_role: String,
    pub currentness: LegalCurrentness,
    pub sources: Vec<LegalCitation>,
    pub disclaimer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCurrentness {
    pub status: String,
    pub reviewed_version_is_current: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCitation {
    pub knowledge_item_id: Uuid,
    pub document_id: Option<Uuid>,
    pub document_title: Option<String>,
    pub authority: Option<String>,
    pub official_id: Option<String>,
    pub eli_id: Option<String>,
    pub source_url: Option<String>,
    pub reviewed_version_id: Option<Uuid>,
    pub version_label: Option<String>,
    pub version_date: Option<NaiveDate>,
    pub retrieved_at: Option<DateTime<Utc>>,
    pub legal_status: Option<String>,
}

struct LegalRule {
    query_terms: &'static [&'static str],
    knowledge_terms: &'static [&'static str],
    detects_ex_form: bool,
}

const LEGAL_RULES: &[LegalRule] = &[
    LegalRule {
        query_terms: &[
            "extranjeria",
            "inmigracion",
            "immigration",
            "spanish immigration",
            "immigration in spain",
            "foreigners office",
            "иммиграц*",
            "миграционн*",
            "иностранец*",
        ],
        knowledge_terms: &["extranjeria", "inmigracion", "immigration"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "residencia",
            "residency",
            "residence",
            "residence permit",
            "spanish residence",
            "вид на жительство",
            "внж",
            "резиденц*",
        ],
        knowledge_terms: &["residencia", "residency", "residence"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "nacionalidad",
            "nationality",
            "citizenship",
            "spanish nationality",
            "spanish citizenship",
            "citizenship in spain",
            "гражданств*",
        ],
        knowledge_terms: &["nacionalidad", "nationality", "citizenship"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &["arraigo"],
        knowledge_terms: &["arraigo"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "familiar de ciudadano espanol",
            "familiares de ciudadanos espanoles",
            "family member of a spanish citizen",
            "family of a spanish citizen",
            "член семьи гражданина испании",
            "семья гражданина испании",
        ],
        knowledge_terms: &[
            "familiar de ciudadano espanol",
            "familiares de ciudadanos espanoles",
            "family member spanish citizen",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "tarjeta comunitaria",
            "tarjeta de familiar de ciudadano de la union",
            "familiar de ciudadano de la ue",
            "familiares de ciudadano de la ue",
            "familiares de ciudadanos de la ue",
            "familiar de ciudadano de la union",
            "familiares de ciudadano de la union",
            "familiares de ciudadanos de la union",
            "familiar ue",
            "eu family card",
            "union citizen family card",
            "карта члена семьи ес",
            "тархета комунитария",
        ],
        knowledge_terms: &[
            "tarjeta comunitaria",
            "familiar ue",
            "familiar de ciudadano de la union",
            "familiares de ciudadanos de la union",
            "eu family card",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "estudiant*",
            "estancia por estudios",
            "visado de estudiante",
            "student visa",
            "student residence",
            "international student",
            "студент*",
            "студенческ*",
            "учебная виза",
        ],
        knowledge_terms: &[
            "estudiant*",
            "estancia por estudios",
            "visado de estudiante",
            "student visa",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "renovacion*",
            "renovar",
            "renewal*",
            "renew residence",
            "продлен*",
            "обновлен*",
        ],
        knowledge_terms: &["renovacion*", "renovar", "renewal*"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "larga duracion",
            "long term residence",
            "permanent residence",
            "долгосрочн*",
            "пмж",
        ],
        knowledge_terms: &[
            "larga duracion",
            "long term residence",
            "permanent residence",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "recurso administrativo",
            "recursos administrativos",
            "recurso de reposicion",
            "recurso de alzada",
            "administrative appeal",
            "административная жалоба",
            "административное обжалование",
        ],
        knowledge_terms: &[
            "recurso administrativo",
            "recursos administrativos",
            "recurso de reposicion",
            "recurso de alzada",
            "administrative appeal",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "formulario ex",
            "formularios ex",
            "ex form",
            "ex forms",
            "форма ex",
            "формы ex",
        ],
        knowledge_terms: &["formulario ex", "formularios ex", "ex form"],
        detects_ex_form: true,
    },
    LegalRule {
        query_terms: &[
            "visado",
            "spanish visa",
            "visa for spain",
            "виза в испанию",
            "испанская виза",
        ],
        knowledge_terms: &["visado", "visa"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "asilo",
            "proteccion internacional",
            "asylum in spain",
            "international protection in spain",
            "убежище в испании",
            "международная защита",
        ],
        knowledge_terms: &["asilo", "proteccion internacional", "asylum"],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "permiso de trabajo",
            "autorizacion de trabajo",
            "work permit in spain",
            "разрешение на работу в испании",
        ],
        knowledge_terms: &[
            "permiso de trabajo",
            "autorizacion de trabajo",
            "work permit",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "toma de huellas",
            "cita previa extranjeria",
            "numero de identidad de extranjero",
            "tarjeta de identidad de extranjero",
            "foreign identity number",
            "nie number",
            "nie",
            "tarjeta tie",
            "tie",
        ],
        knowledge_terms: &[
            "toma de huellas",
            "cita previa",
            "numero de identidad de extranjero",
            "tarjeta de identidad de extranjero",
            "nie",
            "tie",
        ],
        detects_ex_form: false,
    },
    LegalRule {
        query_terms: &[
            "spanish legal procedure",
            "spanish legal procedures",
            "legal procedure in spain",
            "legal procedures in spain",
            "procedimiento legal espanol",
            "procedimiento legal en espana",
            "tramite legal en espana",
            "юридическая процедура в испании",
            "правовая процедура в испании",
        ],
        knowledge_terms: &[
            "extranjeria",
            "inmigracion",
            "immigration",
            "residencia",
            "nacionalidad",
            "arraigo",
            "visado",
        ],
        detects_ex_form: false,
    },
];

pub fn route_question(question: &str) -> QuestionRoute {
    let normalized = normalize(question);
    let has_ex_form = contains_ex_form(&normalized);
    let matched_rules = LEGAL_RULES
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            (rule
                .query_terms
                .iter()
                .any(|term| term_matches(&normalized, term))
                || (rule.detects_ex_form && has_ex_form))
                .then_some(index)
        })
        .collect::<Vec<_>>();

    if matched_rules.is_empty() {
        QuestionRoute::General
    } else {
        QuestionRoute::Legal(LegalRoute {
            language: detect_language(question),
            matched_rules,
        })
    }
}

pub async fn answer<F, Fut>(
    pool: &PgPool,
    question: &str,
    route: &LegalRoute,
    generate: F,
) -> AppResult<LegalAnswer>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = AppResult<String>>,
{
    // ponytail: the reviewed set is intentionally loaded in-process; add indexed multilingual
    // retrieval when approved knowledge volume makes this small deterministic scan inadequate.
    let approved = repository::list_approved_knowledge(pool).await?;
    let selected = select_relevant(approved, question, route);
    if selected.is_empty() {
        return Ok(unavailable_answer(route.language));
    }

    let metadata = metadata_for(route.language, &selected);
    let prompt = build_prompt(question, route.language, &selected);
    let generated = generate(prompt).await?;
    if generated.trim().is_empty() {
        return Err(AppError::Upstream(
            "legal answer generation returned an empty response".to_owned(),
        ));
    }

    let mut answer = generated.trim().to_owned();
    if metadata.currentness.last_checked_at.is_none() {
        answer.push_str("\n\n");
        answer.push_str(unknown_check_note(route.language));
    }
    answer.push_str("\n\n");
    answer.push_str(&metadata.disclaimer);

    Ok(LegalAnswer { answer, metadata })
}

fn select_relevant(
    approved: Vec<KnowledgeItemView>,
    question: &str,
    route: &LegalRoute,
) -> Vec<KnowledgeItemView> {
    let normalized_question = normalize(question);
    let mut ranked = approved
        .into_iter()
        .filter_map(|item| {
            let searchable = normalize(&format!(
                "{} {} {} {} {} {}",
                item.item.procedure_key,
                item.item.topic_key,
                item.item.title_es,
                item.item.canonical_answer_es,
                item.legal_area.as_deref().unwrap_or_default(),
                item.document_title.as_deref().unwrap_or_default(),
            ));
            let mut score = 0;

            for rule_index in &route.matched_rules {
                let rule = &LEGAL_RULES[*rule_index];
                if rule
                    .knowledge_terms
                    .iter()
                    .any(|term| term_matches(&searchable, term))
                    || (rule.detects_ex_form && contains_ex_form(&searchable))
                {
                    score += 100;
                }
            }

            if score == 0 {
                return None;
            }

            score += meaningful_tokens(&normalized_question)
                .filter(|token| word_or_prefix_matches(&searchable, token))
                .count();
            Some((score, item))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| right.0.cmp(&left.0));
    let best_score = ranked.first().map(|(score, _)| *score);
    ranked
        .into_iter()
        .take_while(|(score, _)| Some(*score) == best_score)
        .take(MAX_CONTEXT_ITEMS)
        .map(|(_, item)| item)
        .collect()
}

fn metadata_for(language: AnswerLanguage, items: &[KnowledgeItemView]) -> LegalMetadata {
    let sources = items.iter().map(citation_for).collect::<Vec<_>>();
    let all_current = items
        .iter()
        .all(|item| item.reviewed_version_is_current && !item.is_stale);
    let all_have_check_dates = items.iter().all(|item| item.last_checked_at.is_some());
    let last_checked_at = all_have_check_dates
        .then(|| items.iter().filter_map(|item| item.last_checked_at).min())
        .flatten();
    let status = if !all_current {
        "unavailable"
    } else if last_checked_at.is_some() {
        "current"
    } else {
        "current_lineage_check_date_unknown"
    };

    LegalMetadata {
        language: language.code().to_owned(),
        legal_area: items.iter().find_map(|item| item.legal_area.clone()),
        procedure_key: items
            .iter()
            .map(|item| item.item.procedure_key.trim())
            .find(|value| !value.is_empty())
            .map(str::to_owned),
        reviewed: true,
        reviewer_role: "Legal Reviewer".to_owned(),
        currentness: LegalCurrentness {
            status: status.to_owned(),
            reviewed_version_is_current: all_current,
            last_checked_at,
        },
        sources,
        disclaimer: disclaimer(language).to_owned(),
    }
}

fn citation_for(item: &KnowledgeItemView) -> LegalCitation {
    LegalCitation {
        knowledge_item_id: item.item.id,
        document_id: item.document_id,
        document_title: item.document_title.clone(),
        authority: item.authority.clone(),
        official_id: item.official_id.clone(),
        eli_id: item.eli_id.clone(),
        source_url: item.source_url.clone(),
        reviewed_version_id: item.reviewed_version_id,
        version_label: item.version_label.clone(),
        version_date: item.version_date,
        retrieved_at: item.retrieved_at,
        legal_status: item.legal_status.clone(),
    }
}

fn unavailable_answer(language: AnswerLanguage) -> LegalAnswer {
    let disclaimer = disclaimer(language).to_owned();
    LegalAnswer {
        answer: format!("{}\n\n{}", unavailable_message(language), disclaimer),
        metadata: LegalMetadata {
            language: language.code().to_owned(),
            legal_area: None,
            procedure_key: None,
            reviewed: false,
            reviewer_role: "Legal Reviewer".to_owned(),
            currentness: LegalCurrentness {
                status: "unavailable".to_owned(),
                reviewed_version_is_current: false,
                last_checked_at: None,
            },
            sources: vec![],
            disclaimer,
        },
    }
}

fn build_prompt(question: &str, language: AnswerLanguage, items: &[KnowledgeItemView]) -> String {
    let context = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut labels = vec![format!("Title: {}", item.item.title_es)];
            labels.push(format!("Procedure key: {}", item.item.procedure_key));
            if let Some(area) = &item.legal_area {
                labels.push(format!("Legal area: {area}"));
            }
            if let Some(official_id) = &item.official_id {
                labels.push(format!("Official ID: {official_id}"));
            }
            if let Some(eli_id) = &item.eli_id {
                labels.push(format!("ELI ID: {eli_id}"));
            }
            if let Some(version_label) = &item.version_label {
                labels.push(format!("Version: {version_label}"));
            }
            format!(
                "[{}]\n{}\nCanonical Spanish text:\n{}",
                index + 1,
                labels.join("\n"),
                item.item.canonical_answer_es
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "You format public legal information from the reviewed Legal Core.\n\
         Answer in {language}. The canonical legal basis below is Spanish.\n\
         Use only the APPROVED_CURRENT_MATERIAL. Do not use general knowledge, page context, or assumptions.\n\
         If the material does not answer the question, explicitly say that the reviewed material is insufficient.\n\
         Preserve Spanish article numbers, procedure names, EX form names, official document titles, dates, and legal identifiers exactly.\n\
         Do not invent or translate legal identifiers, facts, dates, citations, titles, URLs, requirements, or deadlines.\n\
         Do not provide definitive legal advice and do not call any reviewer a lawyer or attorney. The public role name is Legal Reviewer.\n\
         Refer to a supplied source only as [1], [2], or [3]. Do not add a disclaimer; the application appends it.\n\n\
         USER_QUESTION\n{question}\n\n\
         APPROVED_CURRENT_MATERIAL\n{context}\n\n\
         Return only the answer text.",
        language = language.prompt_name(),
    )
}

fn detect_language(question: &str) -> AnswerLanguage {
    if question.chars().any(|character| {
        ('\u{0400}'..='\u{04ff}').contains(&character)
            || ('\u{0500}'..='\u{052f}').contains(&character)
    }) {
        return AnswerLanguage::Ru;
    }

    let normalized = normalize(question);
    let spanish_score = language_score(
        &normalized,
        &[
            "que",
            "como",
            "cual",
            "cuando",
            "donde",
            "puedo",
            "necesito",
            "debo",
            "solicitar",
            "requisitos",
            "documentos",
            "plazo",
            "nacionalidad",
            "residencia",
            "extranjeria",
        ],
    );
    let english_score = language_score(
        &normalized,
        &[
            "what",
            "how",
            "which",
            "when",
            "where",
            "can",
            "should",
            "need",
            "apply",
            "requirements",
            "documents",
            "deadline",
            "immigration",
            "residence",
            "nationality",
            "citizenship",
        ],
    );

    if english_score > spanish_score {
        AnswerLanguage::En
    } else if spanish_score > 0 || question.contains('¿') || question.contains('¡') {
        AnswerLanguage::Es
    } else {
        AnswerLanguage::Ru
    }
}

fn language_score(text: &str, markers: &[&str]) -> usize {
    markers
        .iter()
        .filter(|marker| word_or_prefix_matches(text, marker))
        .count()
}

fn meaningful_tokens(text: &str) -> impl Iterator<Item = &str> {
    const STOP_WORDS: &[&str] = &[
        "about",
        "como",
        "cual",
        "cuando",
        "donde",
        "para",
        "pero",
        "puedo",
        "que",
        "the",
        "this",
        "what",
        "when",
        "where",
        "which",
        "with",
        "какие",
        "как",
        "могу",
        "надо",
        "нужно",
        "пожалуйста",
        "что",
        "это",
    ];

    text.split_whitespace().filter(|token| {
        token.chars().count() >= 4 && !STOP_WORDS.iter().any(|stop_word| stop_word == token)
    })
}

fn normalize(value: &str) -> String {
    let lowered = value.to_lowercase();
    let deaccented = lowered
        .replace(['á', 'à', 'ä', 'â'], "a")
        .replace(['é', 'è', 'ë', 'ê'], "e")
        .replace(['í', 'ì', 'ï', 'î'], "i")
        .replace(['ó', 'ò', 'ö', 'ô'], "o")
        .replace(['ú', 'ù', 'ü', 'û'], "u")
        .replace('ñ', "n");
    deaccented
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn term_matches(text: &str, term: &str) -> bool {
    if term.contains(' ') {
        return text.contains(term.trim_end_matches('*'));
    }
    word_or_prefix_matches(text, term)
}

fn word_or_prefix_matches(text: &str, term: &str) -> bool {
    if let Some(prefix) = term.strip_suffix('*') {
        text.split_whitespace().any(|word| word.starts_with(prefix))
    } else {
        text.split_whitespace().any(|word| word == term)
    }
}

fn contains_ex_form(text: &str) -> bool {
    let words = text.split_whitespace().collect::<Vec<_>>();
    words.windows(2).any(|pair| {
        pair[0] == "ex"
            && (pair[1].chars().all(|character| character.is_ascii_digit())
                || matches!(pair[1], "form" | "forms" | "formulario" | "formularios"))
    }) || words.iter().any(|word| {
        word.strip_prefix("ex")
            .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
    })
}

fn unavailable_message(language: AnswerLanguage) -> &'static str {
    match language {
        AnswerLanguage::Es => {
            "Legal Core no dispone todavía de una fuente revisada y vigente que coincida con esta pregunta. Para evitar usar contenido pendiente, rechazado o desactualizado, no elaboraré una respuesta jurídica. Un administrador puede indexar una fuente oficial española y enviarla al proceso Legal Review."
        }
        AnswerLanguage::Ru => {
            "В Legal Core пока нет проверенного и актуального материала по этому вопросу. Чтобы не использовать ожидающий проверки, отклонённый или устаревший контент, я не буду придумывать юридический ответ. Администратор может проиндексировать официальный испанский источник и направить его в процесс Legal Review."
        }
        AnswerLanguage::En => {
            "Legal Core does not yet contain reviewed, current material matching this question. To avoid using pending, rejected, or stale content, I will not fabricate a legal answer. An administrator can index an official Spanish source and send it through the Legal Review process."
        }
    }
}

fn unknown_check_note(language: AnswerLanguage) -> &'static str {
    match language {
        AnswerLanguage::Es => {
            "Vigencia: la versión revisada está marcada como actual, pero no consta la fecha de la última comprobación de la fuente."
        }
        AnswerLanguage::Ru => {
            "Актуальность: проверенная версия отмечена как текущая, но дата последней проверки источника не указана."
        }
        AnswerLanguage::En => {
            "Currentness: the reviewed version is marked current, but the source's last-check date is not recorded."
        }
    }
}

fn disclaimer(language: AnswerLanguage) -> &'static str {
    match language {
        AnswerLanguage::Es => "Información general; no constituye asesoramiento jurídico.",
        AnswerLanguage::Ru => {
            "Информация носит общий характер и не является юридической консультацией."
        }
        AnswerLanguage::En => "General information only; not legal advice.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_requested_legal_topics_in_supported_languages() {
        let legal_questions = [
            "¿Cómo funciona extranjería?",
            "Necesito renovar mi residencia",
            "Nacionalidad por residencia",
            "¿Qué exige el arraigo?",
            "Familiar de ciudadano español",
            "Tarjeta comunitaria para familiar UE",
            "Visado de estudiante",
            "Renovación de autorización",
            "Residencia de larga duración",
            "Recurso administrativo de reposición",
            "¿Dónde presento el EX-10?",
            "¿Qué requisitos hay para familiares de ciudadano de la UE?",
            "Как получить NIE?",
            "What Spanish immigration procedure applies?",
            "How do I obtain a residence permit?",
            "Что нужно для ВНЖ в Испании?",
            "Как продлить студенческую визу?",
        ];

        for question in legal_questions {
            assert!(
                matches!(route_question(question), QuestionRoute::Legal(_)),
                "expected legal route for {question}"
            );
        }
    }

    #[test]
    fn leaves_normal_chat_on_the_existing_route() {
        assert_eq!(
            route_question("Как приготовить паэлью?"),
            QuestionRoute::General
        );
        assert_eq!(
            route_question("What museums are open on Sunday?"),
            QuestionRoute::General
        );
        assert_eq!(route_question("Show me an example"), QuestionRoute::General);
        assert_eq!(
            route_question("Are there student discounts at the museum?"),
            QuestionRoute::General
        );
    }

    #[test]
    fn detects_response_language_without_translating_identifiers() {
        let QuestionRoute::Legal(route) = route_question("What is arraigo?") else {
            panic!("expected legal route");
        };
        assert_eq!(route.language, AnswerLanguage::En);

        let QuestionRoute::Legal(route) = route_question("Что такое arraigo?") else {
            panic!("expected legal route");
        };
        assert_eq!(route.language, AnswerLanguage::Ru);

        let QuestionRoute::Legal(route) = route_question("¿Qué es arraigo?") else {
            panic!("expected legal route");
        };
        assert_eq!(route.language, AnswerLanguage::Es);
    }
}
