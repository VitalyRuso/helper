use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::{fs, path::Path};

pub struct LoadedDocument {
    pub text: String,
    pub document_type: String,
}

pub fn load(path: &Path) -> Result<Option<LoadedDocument>> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let document = match ext.as_str() {
        "txt" | "md" => LoadedDocument {
            text: fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
            document_type: ext,
        },
        "html" | "htm" => LoadedDocument {
            text: html_to_text(&fs::read_to_string(path)?),
            document_type: "html".to_owned(),
        },
        "pdf" => LoadedDocument {
            text: pdf_extract::extract_text(path)
                .with_context(|| format!("extract text from {}", path.display()))?,
            document_type: "pdf".to_owned(),
        },
        _ => return Ok(None),
    };
    Ok(Some(document))
}

fn html_to_text(input: &str) -> String {
    let document = Html::parse_document(input);
    let selector = Selector::parse("body").expect("valid selector");
    document
        .select(&selector)
        .flat_map(|body| body.text())
        .collect::<Vec<_>>()
        .join(" ")
}
