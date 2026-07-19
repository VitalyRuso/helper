pub fn detect_category(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    [
        ("cita-previa", ["cita previa"].as_slice()),
        ("extranjeria", &["extranjería", "extranjeria"]),
        ("empadronamiento", &["empadronamiento"]),
        ("residencia", &["residencia"]),
        ("nie", &["nie"]),
        ("tie", &["tie"]),
        ("nacionalidad", &["nacionalidad"]),
        ("certificado", &["certificado"]),
        ("documentos", &["documentos", "documento"]),
        ("apostilla", &["apostilla"]),
        ("plazo", &["plazo"]),
        ("tasa", &["tasa"]),
        ("solicitud", &["solicitud"]),
        ("recurso", &["recurso"]),
        ("resolucion", &["resolución", "resolucion"]),
    ]
    .into_iter()
    .find(|(_, keywords)| keywords.iter().any(|keyword| lower.contains(keyword)))
    .map(|(slug, _)| slug.to_owned())
}
