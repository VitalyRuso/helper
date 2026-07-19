# Reusable Internal Tools

This folder contains reusable tools that sit next to the current app in `aplicaciones`.

- `shared-contracts` defines shared Rust types and JSON schemas.
- `knowledge-intake-tool` analyzes source text/files and returns draft knowledge candidates for review.
- `assistant-brain-tool` manages assistant profiles, prompts, policies, and change candidates through review.

Nothing here auto-publishes website content or auto-activates assistant behavior.

Examples:

```powershell
cargo run --manifest-path tools/knowledge-intake-tool/Cargo.toml -- analyze-file --path ./docs/demo-cita-previa.md
curl -X POST http://localhost:8000/api/admin/knowledge/sources/scan-docs
curl -X POST http://localhost:8000/api/admin/knowledge/sources/{id}/analyze-now
curl http://localhost:8000/api/admin/knowledge/candidates
curl -X POST http://localhost:8000/api/admin/assistant/notes ^
  -H "Content-Type: application/json" ^
  -d "{\"note_type\":\"idea\",\"title\":\"Better checklist answers\",\"body\":\"Assistant should produce clearer checklists.\"}"
```
