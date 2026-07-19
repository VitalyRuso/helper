# Assistant Brain Tool

Reusable Rust tool for assistant profiles, prompts, policies, and reviewable assistant changes.

It does not directly activate prompt or policy changes. Runtime changes go through candidates first.

```powershell
cargo run --manifest-path tools/assistant-brain-tool/Cargo.toml -- profile create --name "Spain Immigration Helper" --slug spain-immigration-helper
cargo run --manifest-path tools/assistant-brain-tool/Cargo.toml -- prompt validate --file prompt.md
cargo run --manifest-path tools/assistant-brain-tool/Cargo.toml -- candidate create --type prompt_change
cargo run --manifest-path tools/assistant-brain-tool/Cargo.toml -- runtime-config --profile spain-immigration-helper
```
