# Spain Helper AI Audit

## 1. Executive Summary

Verdict: evolve the existing project, but do not ship public legal answers from it yet. The current app is a competent MVP shell: Rust API, React frontend, PostgreSQL, Qdrant, local embeddings, Ollama/OpenAI boundary, simple admin, guest limits, and a non-implemented document analyzer boundary. It is not a serious legal-information system yet.

The dangerous gap is not the stack. The dangerous gap is evidence control. The system can retrieve chunks and ask an LLM to answer, but it cannot prove that claims cite current official law, cannot reject superseded sources, cannot preserve immutable document versions, cannot audit reviewer decisions, and cannot separate approved legal knowledge from raw indexed material.

Keep the app. Build the legal substrate before expanding features.

## 2. Current Architecture Found

Verified from code:

- Main app lives in `E:\RusosenEspana\aplicaciones`, not the repository root.
- API: `E:\RusosenEspana\aplicaciones\apps\api`, Rust/Axum/Tokio/SQLx.
- Web: `E:\RusosenEspana\aplicaciones\apps\web`, Vite/React/TypeScript/Tailwind/TanStack Query.
- Local helper crates live beside the app under `E:\RusosenEspana\tools`.
- `E:\RusosenEspana\aplicaciones\docker-compose.yml` runs PostgreSQL, Qdrant, API, and web.
- `E:\RusosenEspana\aplicaciones\docs\demo-cita-previa.md` is the only document found under the app docs folder.
- `E:\RusosenEspana\aplicaciones\.env` exists locally; `.env.example` is tracked-looking project material and contains development defaults.
- `git status --short` failed from both `E:\RusosenEspana` and `E:\RusosenEspana\aplicaciones` with `fatal: not a git repository (or any of the parent directories): .git`, despite empty `.git` directories being present.

API modules found:

- Routes: `apps/api/src/routes`.
- Services: `apps/api/src/services`.
- RAG: `apps/api/src/rag`.
- Migrations: `apps/api/migrations/0001_initial.sql`, `apps/api/migrations/0002_internal_tools.sql`.
- Tool adapters: `apps/api/src/tool_adapters`.
- Config/state/security/error handling: `apps/api/src/config.rs`, `apps/api/src/state.rs`, `apps/api/src/security.rs`, `apps/api/src/error.rs`.

Web routes found in `apps/web/src/app/App.tsx`:

- Public pages: home, guides, KB, search, assistant, document analyzer, pricing, about, legal.
- Admin pages: dashboard, knowledge sources/candidates, assistant profiles/candidates/notes.

Inferred from README:

- Intended default LLM is Ollama.
- FastEmbed is the intended default embedding provider.
- `local-hashing-v1` is a development fallback.

## 3. What Works And Should Be Kept

- Keep the Rust API structure. `apps/api/src/routes/mod.rs` is simple and understandable.
- Keep SQLx and migrations. The schema is small and explicit.
- Keep PostgreSQL as the system-of-record.
- Keep Qdrant for vector search, but change payload identity and ingestion semantics before legal use.
- Keep FastEmbed local embeddings. `apps/api/src/rag/embeddings.rs` has a clean enough provider boundary.
- Keep the LLM boundary in `apps/api/src/rag/llm.rs`; adding `LocalControlCenterProvider` should be a small branch plus provider module.
- Keep the document analyzer as a 501 boundary in `apps/api/src/routes/documents.rs` until there is safe upload/storage/review design.
- Keep admin shell as a starting point, not as the final reviewer workflow.
- Keep helper crates as scaffolding boundaries: `knowledge-intake-tool`, `assistant-brain-tool`, and `shared-contracts`.
- Keep the rule that direct assistant prompt/policy activation is rejected through candidates.
- Keep the empty-Qdrant refusal in `apps/api/src/rag/agent.rs`; refusing to fake an answer is the right instinct.

## 4. Prototype / Weak Areas

- `apps/api/src/main.rs` uses `CorsLayer::permissive()` and ignores configured `CORS_ORIGINS`.
- `POST /api/rag/reindex` in `apps/api/src/routes/rag.rs` has no admin auth and clears/recreates the Qdrant collection.
- Admin auth is a single username/password and a random in-memory token in `apps/api/src/main.rs` / `apps/api/src/security.rs`; tokens disappear on restart and cannot be audited or revoked per user.
- The web stores the admin token in localStorage in `apps/web/src/pages/AdminPage.tsx` and `apps/web/src/pages/AdminToolsPages.tsx`.
- Guest limits depend on caller-supplied `session_id`; `apps/web/src/lib/session.ts` stores it in localStorage, so bypass is trivial.
- Access keys are SHA-256 hashes only, seeded from env in `apps/api/src/security.rs`; no salt, no expiry, no owner, no scope.
- `ADMIN_PASSWORD=change-me` and demo access keys exist in `.env.example`.
- `apps/api/src/rag/indexer.rs` clears Qdrant on reindex but only appends to `source_documents`; stale DB metadata accumulates.
- Qdrant payload stores full text chunks and local `source_file` paths in `apps/api/src/rag/qdrant_store.rs`.
- `source_documents` has no immutable version, checksum, source authority, effective date, section id, or relation to Qdrant point ids.
- The active assistant policy fields `retrieval_top_k`, `require_sources`, `allow_llm_without_sources`, and `allowed_collection` are loaded in `assistant_brain_adapter.rs`, but the chat path only uses `system_prompt`.
- `knowledge-intake-tool` is keyword extraction and draft generation, not legal analysis.
- The search endpoint is simple `ILIKE`; fine for MVP content search, not legal retrieval.

## 5. Legal Product Blockers

Confirmed blockers:

- No official source registry.
- No immutable legal document versions.
- No section-level canonical model.
- No deterministic legal diff model.
- No claim/evidence model.
- No citation verifier.
- No superseded-source guard.
- No reviewer identity or durable audit trail.
- No answer run trace.
- No human handoff model.
- No separation between raw indexed docs and approved legal knowledge.
- Public chat can use whatever is indexed, not only reviewed and approved legal sources.

Likely blockers:

- No currentness model for Spanish/EU law.
- No canonical Spanish answer layer before translation.
- No protection against hallucinated article/form/deadline citations beyond prompting.
- No procedure-specific risk classification.

Not observed:

- No BOE, Migraciones, EUR-Lex ingestion code.
- No uploaded document storage flow.
- No live official-source monitoring.

## 6. Data Model Gaps

Current tables:

- Content: `categories`, `articles`, `guides`.
- Sessions/access/chat: `sessions`, `access_keys`, `chat_messages`.
- RAG metadata: `source_documents`.
- Admin/helper scaffolding: `data_sources`, `analysis_jobs`, `extracted_facts`, `content_candidates`, `indexing_jobs`, `assistant_profiles`, `assistant_prompt_versions`, `assistant_policies`, `assistant_change_candidates`, `assistant_test_runs`, `assistant_notes`.

Missing for Official Source Registry:

- `legal_sources`, `source_fetch_runs`, `source_records`.
- Current `data_sources` is too loose: no source authority, polling config, canonical URL, jurisdiction, language, fetch hash, or fetch status history.

Missing for Document Versioning:

- `legal_documents`, `document_versions`, `document_sections`, `document_relations`.
- Current `source_documents` is one imported row per file indexing pass, not an immutable legal-document ledger.

Missing for Change Detection:

- `document_diffs`, `legal_changes`.
- Current reindex has no old/new comparison.

Missing for AI Legal Analysis:

- `ai_analyses`, `analysis_claims`, `claim_evidence`.
- Current `analysis_jobs` stores opaque output JSON, not verifiable claims.

Missing for Citation Verification:

- Stored source sections with stable IDs, version IDs, offsets, official identifiers, current/superseded status.
- A verifier that rejects unsupported claims, wrong versions, and stale citations.

Missing for Human Review:

- `review_tasks`, `review_decisions`, `reviewer_notes`, `audit_events`.
- Current candidates have status and note only.

Missing for Approved Knowledge Base:

- `knowledge_items`, `knowledge_versions`, `legal_topics`, `procedures`.
- Current articles/guides can be reused as public content, not as the legal knowledge source of truth.

Missing for Multilingual Chat:

- `answer_runs`, canonical Spanish answer, translated presentations, source trace, preserved Spanish terms/forms/articles.

Missing for Human Handoff:

- `chat_sessions`, richer `chat_messages`, `human_handoffs`, `answer_runs`.
- Current `chat_messages` has role/content/sources only.

## 7. RAG And Search Assessment

What is reusable:

- `apps/api/src/rag/embeddings.rs`: FastEmbed/local-hashing provider abstraction.
- `apps/api/src/rag/qdrant_store.rs`: Qdrant HTTP client pattern and vector dimension guard.
- `apps/api/src/rag/indexer.rs`: simple docs walking and indexing flow.
- `apps/api/src/rag/loaders.rs`: basic md/txt/html/pdf text extraction.
- `apps/api/src/rag/splitter.rs`: small chunking utility.

What must change:

- Do not let public or unauthenticated endpoints clear/reindex legal vectors.
- Index legal sections, not arbitrary character chunks.
- Store stable `document_version_id` and `section_id` in Qdrant payload.
- Stop storing local filesystem paths as public source identity.
- Stop treating Qdrant payload as the legal evidence store.
- Do not answer from raw indexed docs unless source and version are approved/current.
- Add retrieval filters for jurisdiction, legal topic, language, document status, currentness, and approved-only use.
- Replace arbitrary score threshold `> 0.05` with evaluated retrieval behavior.
- Add citation verification after generation.

Current search:

- `apps/api/src/services/search_service.rs` uses parameterized SQL with `ILIKE`; acceptable for public article/guide search.
- It is not legal search.

## 8. AI Provider Assessment

Current provider boundary:

- `apps/api/src/rag/llm.rs` switches on `LLM_PROVIDER`.
- `apps/api/src/rag/ollama.rs` calls `/api/generate`.
- `apps/api/src/rag/openai.rs` calls chat completions.

Reusable:

- The branch-based provider boundary is enough for now.
- Add `LocalControlCenterProvider` as a third provider module and one new config group.

Must fix before serious use:

- Runtime assistant policies are not enforced in the chat path except `system_prompt`.
- No timeout/retry/circuit-breaker policy per provider.
- No structured response format.
- No answer trace.
- No claim extraction or verification pass.
- No deterministic refusal if the model cites unsupported material.

## 9. Admin And Review Workflow Assessment

Reusable:

- `apps/web/src/pages/AdminToolsPages.tsx` provides a basic admin shell for sources, candidates, assistant profiles, candidates, and notes.
- `apps/api/src/routes/admin_knowledge.rs` and `apps/api/src/routes/admin_assistant.rs` already require admin bearer tokens.
- Candidate approval creates draft unpublished articles, which is safer than direct public publication.

Not enough:

- No reviewer accounts.
- No roles.
- No MFA/session lifecycle.
- No task assignment.
- No reviewer identity on decisions.
- No immutable audit events.
- No two-person approval for high-risk changes.
- No legal source/version viewer.
- No diff viewer.
- No "approved for public answers" gate.

## 10. Chat And Multilingual Assessment

Current behavior:

- `apps/api/src/routes/chat.rs` accepts `message`, `session_id`, and optional `page_context`.
- The assistant answers in Russian by default through `apps/api/src/rag/prompts.rs`.
- It logs user/assistant messages and returned source chunks.
- Guest limit is enforced before answering and incremented after answer generation.

Reusable:

- Basic chat API shape.
- Existing `chat_messages` can be a migration source.
- The refusal-on-empty-RAG behavior.

Missing:

- Canonical Spanish answer.
- Russian/English/Spanish presentation layer.
- Preserved legal terms, article numbers, form identifiers, and exact official source labels.
- Answer runs.
- Claim/evidence rows.
- Citation verification.
- Human handoff.
- User identity/account model.
- Per-session risk state.
- Human author messages.

Danger:

- Sources returned to the frontend are chunks, not verified citations.
- Prompt rules are not enforcement.
- Legal answer safety cannot depend on "Answer only from context."

## 11. Security And Privacy Risks

Confirmed:

- Public `POST /api/rag/reindex` can clear/rebuild Qdrant if API is reachable.
- CORS is permissive in code.
- Admin token is stored in localStorage.
- Admin auth is single shared credential plus in-memory bearer token.
- `.env` exists locally; `.env.example` contains weak development defaults.
- Guest limit is bypassable by changing `session_id`.
- Qdrant payload stores chunk text and local source paths.
- Chat logs store raw message content.
- No rate limiting observed.
- No reviewer audit trail observed.
- No source versioning/currentness guard observed.
- No citation verification observed.

Likely:

- Brute-force risk on admin login and access-key unlock.
- Personal data will enter chat logs if used by real immigrants.
- Uploaded documents would contain sensitive personal data once analyzer is implemented.
- Admin endpoints are too broad for production even though bearer-protected.

Possible:

- Local file path leakage through returned sources.
- Raw official/non-official/user-provided sources could be mixed in Qdrant.
- Direct use of non-official material in answers if indexed.

Not observed:

- SQL injection in inspected query paths; user input is parameterized.
- Implemented upload path traversal; document analyzer is 501.
- Secrets committed to git could not be verified because git metadata is not functional in this workspace.

## 12. Testing And Build Results

Commands run:

- `cargo test` in `E:\RusosenEspana\aplicaciones\apps\api`: passed. 5 unit tests, 3 integration tests, doc tests passed.
- `cargo check` in `E:\RusosenEspana\aplicaciones\apps\api`: passed.
- `npm.cmd run build` in `E:\RusosenEspana\aplicaciones\apps\web`: passed. Vite built production bundle.
- `cargo test` in `E:\RusosenEspana\tools\knowledge-intake-tool`: passed. 6 tests.
- `cargo test` in `E:\RusosenEspana\tools\assistant-brain-tool`: passed. 6 tests.
- `cargo test` in `E:\RusosenEspana\tools\shared-contracts`: passed. 0 tests.

Commands with issues:

- `git status --short` in `E:\RusosenEspana\aplicaciones`: failed with `fatal: not a git repository (or any of the parent directories): .git`.
- `git status --short` in `E:\RusosenEspana`: failed with the same error.

Not run:

- Docker services were not started.
- API server was not started.
- Indexing was not run because it requires live PostgreSQL/Qdrant and may download FastEmbed model files on first use.
- `npm test` was not run because `apps/web/package.json` has no test script.

## 13. Recommended Refactor Path

Recommendation: A. evolve the existing Spain Helper project.

Option A: evolve existing project

- Pros: least migration cost; existing stack is sane; current API/web/admin/RAG/provider boundaries are reusable.
- Cons: legal foundation must be added carefully; current MVP tables cannot be stretched into legal versioning.
- Time cost: lowest.
- Technical risk: moderate.
- Migration complexity: moderate.

Option B: create a new backend beside it and reuse frontend/RAG pieces

- Pros: clean legal core without MVP schema compromises.
- Cons: duplicate services, auth, deployments, API contracts, and admin concepts.
- Time cost: medium.
- Technical risk: medium-high.
- Migration complexity: high.

Option C: create a new app and migrate useful modules

- Pros: cleanest architecture on paper.
- Cons: wastes working MVP shell; biggest delay; highest chance of rebuilding solved plumbing.
- Time cost: highest.
- Technical risk: high.
- Migration complexity: highest.

Use A, but add a new legal domain inside the existing API instead of mutating `articles`, `guides`, and `source_documents` into things they are not.

## 14. First 10 Engineering Tasks

1. Lock down dangerous endpoints.
   - Purpose: prevent unauthenticated mutation of legal/RAG state.
   - Files/modules likely affected: `apps/api/src/routes/rag.rs`, `apps/api/src/security.rs`, `apps/web/src/api/client.ts`.
   - Risk: low.
   - Expected tests: unauthenticated reindex returns 401; admin reindex works.

2. Replace permissive CORS with configured origins.
   - Purpose: stop cross-origin access from everywhere.
   - Files/modules likely affected: `apps/api/src/main.rs`, `apps/api/src/config.rs`.
   - Risk: low.
   - Expected tests: config parsing; allowed/disallowed origin behavior if using route-level test.

3. Add legal source registry tables.
   - Purpose: define official source identity before ingestion.
   - Files/modules likely affected: new migration under `apps/api/migrations`, new `legal_sources` service/routes.
   - Risk: medium.
   - Expected tests: insert/list source, unique canonical source key.

4. Add immutable document versioning.
   - Purpose: make every source snapshot addressable and auditable.
   - Files/modules likely affected: migrations, legal document service, ingestion scaffolding.
   - Risk: medium.
   - Expected tests: same source can have multiple versions; checksums deduplicate exact repeats.

5. Add section-level parsing model.
   - Purpose: cite sections, not arbitrary chunks.
   - Files/modules likely affected: loaders/indexer, new `document_sections` table.
   - Risk: medium.
   - Expected tests: parser creates stable ordered sections with offsets.

6. Change Qdrant payload to section/version IDs.
   - Purpose: make vectors point back to PostgreSQL evidence.
   - Files/modules likely affected: `apps/api/src/rag/qdrant_store.rs`, `apps/api/src/rag/indexer.rs`.
   - Risk: medium.
   - Expected tests: search result resolves through DB to approved/current section.

7. Add deterministic diff records.
   - Purpose: detect what changed before AI analysis.
   - Files/modules likely affected: new diff service, migrations.
   - Risk: medium.
   - Expected tests: old/new text produces stable section diff rows.

8. Add analysis claim/evidence tables.
   - Purpose: stop storing legal analysis as opaque JSON.
   - Files/modules likely affected: migrations, analysis service, helper contracts.
   - Risk: medium-high.
   - Expected tests: every claim must have at least one evidence section or be marked unsupported.

9. Add citation verifier.
   - Purpose: fail unsupported, wrong-version, and superseded citations.
   - Files/modules likely affected: new verifier module, chat/analysis pipeline.
   - Risk: high.
   - Expected tests: unsupported claim fails; superseded section fails; current cited section passes.

10. Add review/audit workflow.
    - Purpose: make human approval durable and attributable.
    - Files/modules likely affected: admin routes, admin UI, migrations for `review_tasks`, `review_decisions`, `reviewer_notes`, `audit_events`.
    - Risk: high.
    - Expected tests: reviewer decision writes immutable audit event and gates public answer eligibility.

## 15. Do Not Do Yet

- Do not implement public uploaded-document analysis.
- Do not add BOE/Migraciones/EUR-Lex crawlers before source/version tables exist.
- Do not redesign the frontend.
- Do not add another vector database.
- Do not add agentic autonomous source monitoring.
- Do not expose public legal answers from unreviewed RAG.
- Do not translate answers before canonical Spanish answer tracing exists.
- Do not build a full CMS before legal knowledge versioning exists.
- Do not add payment/subscription complexity before safety gates are real.
- Do not trust prompt text as a legal safety mechanism.

## 16. Final Verdict

Spain Helper AI is a good MVP skeleton and a bad legal platform today. Keep the stack, keep the boundaries, and stop expanding surface area until the legal evidence model exists. The first serious product is not a prettier chatbot; it is a boring chain from official source record to immutable version to section to verified claim to reviewed answer. Build that, then let the assistant speak.
