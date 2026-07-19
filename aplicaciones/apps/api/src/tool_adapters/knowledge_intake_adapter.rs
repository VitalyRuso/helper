use anyhow::Result;
use knowledge_intake_tool::{analyze_source, intake::is_supported_path, KnowledgeAnalysisOutput};
use shared_contracts::KnowledgeSourceInput;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn analyze(input: KnowledgeSourceInput) -> Result<KnowledgeAnalysisOutput> {
    Ok(analyze_source(input)?)
}

pub fn scan_docs(docs_dir: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    scan(Path::new(docs_dir), &mut paths)?;
    Ok(paths)
}

fn scan(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan(&path, paths)?;
        } else if is_supported_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}
