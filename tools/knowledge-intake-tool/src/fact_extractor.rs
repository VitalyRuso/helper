use shared_contracts::{ExtractedFact, FactType};

pub fn extract_facts(text: &str) -> Vec<ExtractedFact> {
    text.lines()
        .flat_map(|line| line.split(['.', ';']))
        .map(str::trim)
        .filter(|line| line.len() > 8)
        .filter_map(fact_from_line)
        .collect()
}

fn fact_from_line(line: &str) -> Option<ExtractedFact> {
    let lower = line.to_lowercase();
    let fact_type = if has(
        &lower,
        &[
            "pasaporte",
            "documento",
            "certificado",
            "nie",
            "tie",
            "formulario",
        ],
    ) {
        FactType::Document
    } else if has(
        &lower,
        &[
            "plazo", "deadline", "fecha", "días", "dias", "meses", "hasta",
        ],
    ) {
        FactType::Deadline
    } else if has(&lower, &["tasa", "fee", "importe", "euros", "€"]) {
        FactType::Fee
    } else if has(
        &lower,
        &["debe", "requisito", "required", "obligatorio", "necesario"],
    ) {
        FactType::Requirement
    } else if has(
        &lower,
        &["paso", "solicitud", "presentar", "cita previa", "tramitar"],
    ) {
        FactType::ProcedureStep
    } else if has(
        &lower,
        &["riesgo", "warning", "importante", "recurso", "deneg"],
    ) {
        FactType::Warning
    } else if has(
        &lower,
        &["teléfono", "telefono", "email", "correo", "contacto"],
    ) {
        FactType::Contact
    } else if has(&lower, &["boe", "sede", "fuente", "source", "oficial"]) {
        FactType::SourceReference
    } else {
        return None;
    };

    Some(ExtractedFact {
        title_ru: title_for(&fact_type).to_owned(),
        text_ru: line.to_owned(),
        original_text: line.to_owned(),
        confidence: 0.7,
        source_location: None,
        fact_type,
    })
}

fn has(input: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| input.contains(needle))
}

fn title_for(fact_type: &FactType) -> &'static str {
    match fact_type {
        FactType::Requirement => "Требование",
        FactType::Deadline => "Срок",
        FactType::Fee => "Пошлина",
        FactType::Document => "Документ",
        FactType::ProcedureStep => "Шаг процедуры",
        FactType::Warning => "Риск",
        FactType::Definition => "Определение",
        FactType::Contact => "Контакт",
        FactType::SourceReference => "Источник",
        FactType::Unknown => "Факт",
    }
}
