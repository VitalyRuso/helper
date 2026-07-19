use crate::{KnowledgeToolError, KnowledgeToolResult};
use shared_contracts::{KnowledgeSourceInput, SourceType};
use std::{fs, path::Path};

pub fn source_text(input: &KnowledgeSourceInput) -> KnowledgeToolResult<String> {
    if let Some(text) = input
        .raw_text
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Ok(strip_html_if_needed(text, input));
    }

    match input.source_type {
        SourceType::File => {
            let path = input
                .original_path
                .as_deref()
                .ok_or(KnowledgeToolError::MissingText)?;
            load_file(Path::new(path))
        }
        SourceType::Url => input
            .source_url
            .clone()
            .or_else(|| Some(input.title.clone()))
            .ok_or(KnowledgeToolError::MissingText),
        SourceType::PastedText | SourceType::ManualNote => Err(KnowledgeToolError::MissingText),
    }
}

pub fn load_file(path: &Path) -> KnowledgeToolResult<String> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "md" | "txt" => Ok(fs::read_to_string(path)?),
        "html" | "htm" => Ok(strip_html(&fs::read_to_string(path)?)),
        // ponytail: no PDF dependency in the standalone tool; wire pdf-extract here if this crate needs it.
        "pdf" => Err(KnowledgeToolError::UnsupportedFileType("pdf".to_owned())),
        other => Err(KnowledgeToolError::UnsupportedFileType(other.to_owned())),
    }
}

pub fn is_supported_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "md" | "txt" | "html" | "htm"
    )
}

fn strip_html_if_needed(text: &str, input: &KnowledgeSourceInput) -> String {
    if input
        .original_path
        .as_deref()
        .is_some_and(|path| path.ends_with(".html") || path.ends_with(".htm"))
    {
        strip_html(text)
    } else {
        text.to_owned()
    }
}

fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
