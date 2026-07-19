# Knowledge Intake Tool

Deterministic Rust tool for turning source data into facts and draft content candidates.

It does not crawl, scrape protected sites, auto-publish, or invent legal requirements.

```powershell
cargo run --manifest-path tools/knowledge-intake-tool/Cargo.toml -- analyze --text "Para NIE debe presentar pasaporte."
cargo run --manifest-path tools/knowledge-intake-tool/Cargo.toml -- analyze-file --path ./docs/demo.md
cargo run --manifest-path tools/knowledge-intake-tool/Cargo.toml -- scan-docs --docs ./docs --output json
```
