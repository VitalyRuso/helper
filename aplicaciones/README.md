# Spain Helper AI

Russian-first web platform for immigrants, expats, and newcomers in Spain. The MVP is a normal web portal with guides, knowledge base, search, an integrated AI assistant, guest limits, access-key unlock, PostgreSQL, Qdrant, Ollama, and Docker.

This is not legal advice. The assistant is an informational tool, not a lawyer, gestor, immigration officer, or public authority. For risky or final decisions, check the official source or a licensed professional.

## Architecture

- `apps/web`: Vite, React, TypeScript, Tailwind, TanStack Query, reusable chat widget, admin shell.
- `apps/api`: Rust, Axum, Tokio, SQLx, PostgreSQL, Qdrant HTTP API, Ollama/OpenAI chat provider boundary.
- `docs`: local official documents to index.
- `postgres`: app data, sessions, access keys, content, chat logs, document metadata.
- `qdrant`: vectors for local RAG.
- `ollama`: default local LLM provider, expected on host at `http://localhost:11434`.

Embeddings stay local. The default provider is `fastembed` with `intfloat/multilingual-e5-small`. It downloads the ONNX model on first indexing/query use and then runs local inference from cache. `local-hashing-v1` remains only as an explicit development fallback via `EMBEDDING_PROVIDER=local-hashing`.

## Run Locally

```bash
cp .env.example .env
make db
make api
make web
```

Open:

- Web: `http://localhost:3000`
- API health: `http://localhost:8000/health`
- Qdrant: `http://localhost:6333`

Run Ollama separately:

```bash
ollama pull llama3.1
ollama serve
```

## Index Documents

Put `.pdf`, `.txt`, `.md`, or `.html` files in `docs/`, then run:

```bash
make index
```

or call:

```bash
curl -X POST http://localhost:8000/api/rag/reindex
```

If Qdrant is empty, the assistant refuses to fake an answer and asks you to index documents first.

## Chat And Access Keys

Guests get 3 answered AI questions per `session_id`. After that, the API returns:

```text
Бесплатный лимит закончился. Введите ключ доступа или оформите подписку.
```

Unlock in chat:

```text
/key demo123
```

Valid keys come from `ACCESS_KEYS` in `.env`; the database stores only hashes.

Commands:

- `/help`
- `/status`
- `/key ACCESS_KEY`

## Admin

Admin login uses:

- `ADMIN_USERNAME`
- `ADMIN_PASSWORD`

The current admin UI has login, stats, RAG status, and reindex. Content CRUD endpoints exist under:

- `POST /api/admin/categories`
- `PUT /api/admin/categories/:id`
- `DELETE /api/admin/categories/:id`
- `POST /api/admin/articles`
- `PUT /api/admin/articles/:id`
- `DELETE /api/admin/articles/:id`
- `POST /api/admin/guides`
- `PUT /api/admin/guides/:id`
- `DELETE /api/admin/guides/:id`

The API seeds base categories and safe generic guides on startup.

## Docker

```bash
cp .env.example .env
make docker-up
```

Services:

- `web`: `3000`
- `api`: `8000`
- `postgres`: `5432`
- `qdrant`: `6333`

`./docs` is mounted into the API container. If Ollama runs on the host, Docker uses `http://host.docker.internal:11434`.

FastEmbed vector dimensions depend on `EMBEDDING_MODEL`. If you change the model after indexing, reset the Qdrant collection or volume and re-run indexing:

```bash
docker compose down
docker volume rm files-mentioned-by-the-user-you_qdrant_data
docker compose up -d postgres qdrant api
make index
```

## API Surface

- `GET /health`
- `GET /api/categories`
- `GET /api/categories/:slug`
- `GET /api/articles`
- `GET /api/articles/:slug`
- `GET /api/articles?category=...`
- `GET /api/guides`
- `GET /api/guides/:slug`
- `GET /api/search?q=...`
- `POST /api/chat`
- `POST /api/access/unlock`
- `GET /api/rag/status`
- `POST /api/rag/reindex`
- `POST /api/admin/login`
- `GET /api/admin/stats`
- `POST /api/documents/analyze` returns `501` until implemented.

## Future Browser Extension

Add `apps/browser-extension` later as a Chrome Manifest V3 extension that calls the same backend:

- side panel assistant
- current page DOM summary
- Russian explanations of Spanish government pages
- suggested actions with user confirmation
- no automatic form submission

The MVP intentionally does not ship broken extension code.

## Future Document Analyzer

The frontend has `/document-analyzer`; the backend boundary is `POST /api/documents/analyze`. Fill that endpoint with upload handling, file extraction, RAG-backed analysis, and explicit user consent before storing documents.

## Known Limitations

- The MVP does not include a full visual CMS yet.
- `fastembed` downloads model files on first use, so the first indexing run needs network access.
- `local-hashing-v1` is deterministic local vector retrieval, not semantic-quality model embeddings, and is not the default.
- RAG quality depends on indexed official documents in `docs/`.
- OpenAI is optional for chat only; embeddings remain local.
