# Spain Helper AI

Spain Helper AI is a Russian-first information portal for immigrants and newcomers in Spain. The MVP includes public guides and articles, search, an AI assistant, guest access limits, admin knowledge tools, and a reviewed Legal Core for Spanish immigration procedures.

The service provides general information, not legal advice. For important decisions, verify the official source or consult a qualified professional.

## What works

- Public categories, articles, guides, detail pages, and search.
- Chat with Ollama or the existing OpenAI provider boundary.
- PostgreSQL-backed sessions, guest limits, access keys, and chat history.
- Qdrant document indexing and RAG for non-legal questions.
- Legal-question routing that never falls back to raw RAG.
- Legal answers based only on approved knowledge with a current reviewed document lineage.
- Explicit refusal when matching reviewed/current legal material is unavailable.
- Admin login, statistics, RAG status/reindex, knowledge intake, assistant configuration candidates, and Legal Review.
- Development fixture for demonstrating Legal Review approval/rejection.

The document analyzer page is intentionally disabled because safe upload analysis is not implemented.

## Stack

- `apps/web`: React, TypeScript, Vite, Tailwind, TanStack Query.
- `apps/api`: Rust, Axum, Tokio, SQLx.
- PostgreSQL: content, sessions, admin workflows, Legal Core, and audit data.
- Qdrant: vector search for the general RAG path.
- Ollama by default, with the existing optional OpenAI chat boundary.
- FastEmbed local embeddings by default.

## Run with Docker

Requirements: Docker Desktop/Compose and, for generated chat answers, Ollama running on the host.

```powershell
Copy-Item .env.example .env
docker compose up --build --wait
docker compose ps
```

Open:

- Web: `http://localhost:3000`
- API health: `http://localhost:8000/health`
- Qdrant: `http://localhost:6333`
- Admin: `http://localhost:3000/admin`
- Legal Review: `http://localhost:3000/admin/legal`

Ollama runs outside this Compose stack:

```powershell
ollama pull llama3.1
ollama serve
```

Stop the stack without deleting data volumes:

```powershell
docker compose down
```

## Run services separately

Start PostgreSQL and Qdrant:

```powershell
docker compose up -d postgres qdrant
```

Start the API:

```powershell
cd apps/api
cargo run
```

Start the web app in another terminal:

```powershell
cd apps/web
npm.cmd install
npm.cmd run dev -- --host 0.0.0.0
```

## Environment

Copy `.env.example` to `.env`. The file contains development-only defaults; change database, access-key, and admin credentials outside local development.

Required:

- `DATABASE_URL`

Main runtime settings:

- `API_HOST`, `API_PORT`, `APP_ENV`
- `VITE_API_URL`, `WEB_PORT`
- `QDRANT_URL`, `QDRANT_COLLECTION`
- `DOCS_DIR`, `CHUNK_SIZE`, `CHUNK_OVERLAP`, `TOP_K`
- `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`
- `LLM_PROVIDER`, `OLLAMA_BASE_URL`, `OLLAMA_MODEL`
- `OPENAI_API_KEY`, `OPENAI_MODEL` when `LLM_PROVIDER=openai`
- `ACCESS_KEYS`, `GUEST_QUESTION_LIMIT`
- `ADMIN_USERNAME`, `ADMIN_PASSWORD`
- `CORS_ORIGINS`, `LOG_LEVEL`

## Index demo documents

Files in `docs/` are indexing inputs; they are not automatically trusted or official legal sources.

With the API environment configured:

```powershell
cd apps/api
cargo run --bin index_docs
```

The same general RAG reindex operation is available after admin login. It requires the admin bearer token and is not a public endpoint.

## Legal Review demo

1. Log in at `/admin`.
2. Open `/admin/legal`.
3. Use **Run fixture** in development.
4. Open the pending task and approve or reject it with a reviewer note.
5. Approved material appears under **Approved Knowledge**.
6. A matching public legal question uses only that approved/current material and shows Legal Reviewer, source, version, and currentness metadata.

Pending, rejected, stale, future-effective, disabled-source, or otherwise unreviewed legal material is excluded. If no matching safe item exists, chat returns a localized refusal instead of using raw Qdrant chunks.

## Verify

Backend:

```powershell
cd apps/api
cargo fmt --check
cargo check
cargo test
```

Frontend:

```powershell
cd apps/web
npm.cmd run build
```

Compose and smoke checks:

```powershell
docker compose config
docker compose up --build --wait
docker compose ps
Invoke-RestMethod http://localhost:8000/health
Invoke-RestMethod http://localhost:8000/api/categories
docker compose logs --tail=200
docker compose down
```

## Known limitations

- Ollama is not bundled in Compose and must run separately for generated answers.
- FastEmbed downloads its model on first embedding use.
- The demo document under `docs/` is not an official legal source.
- Source-specific indexing jobs have no worker; use the reviewed Legal Core pipeline for legal material and admin RAG reindex for files under `docs/`.
- No document upload analyzer, payment system, browser extension, or live BOE/Migraciones/EUR-Lex monitoring is included.
- Admin authentication is suitable for a local/demo MVP, not a production multi-user control plane.
