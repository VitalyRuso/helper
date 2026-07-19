use knowledge_intake_tool::{
    analyze_source,
    intake::{is_supported_path, load_file},
    output::print_json,
    KnowledgeSourceInput, SourceType, TrustLevel,
};
use serde_json::json;
use std::{env, fs, path::Path};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("analyze") => {
            let text = option(&args, "--text").unwrap_or_default();
            let output = analyze_source(KnowledgeSourceInput {
                title: "Pasted text".to_owned(),
                source_type: SourceType::PastedText,
                original_path: None,
                source_url: None,
                raw_text: Some(text),
                trust_level: TrustLevel::Unknown,
            })?;
            print_json(json!(output))?;
        }
        Some("analyze-file") => {
            let path = option(&args, "--path").ok_or("--path is required")?;
            let output = analyze_source(KnowledgeSourceInput {
                title: Path::new(&path)
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or("Document")
                    .to_owned(),
                source_type: SourceType::File,
                original_path: Some(path),
                source_url: None,
                raw_text: None,
                trust_level: TrustLevel::Unknown,
            })?;
            print_json(json!(output))?;
        }
        Some("scan-docs") => {
            let docs = option(&args, "--docs").unwrap_or_else(|| "./docs".to_owned());
            let mut outputs = Vec::new();
            scan(Path::new(&docs), &mut outputs)?;
            print_json(json!(outputs))?;
        }
        _ => {
            eprintln!("usage: knowledge-intake-tool analyze --text ... | analyze-file --path ... | scan-docs --docs ... --output json");
            std::process::exit(2);
        }
    }
    Ok(())
}

fn scan(
    dir: &Path,
    outputs: &mut Vec<knowledge_intake_tool::KnowledgeAnalysisOutput>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan(&path, outputs)?;
        } else if is_supported_path(&path) {
            let text = load_file(&path)?;
            outputs.push(analyze_source(KnowledgeSourceInput {
                title: path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or("Document")
                    .to_owned(),
                source_type: SourceType::File,
                original_path: Some(path.to_string_lossy().to_string()),
                source_url: None,
                raw_text: Some(text),
                trust_level: TrustLevel::Unknown,
            })?);
        }
    }
    Ok(())
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
