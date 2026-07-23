# Spain Helper AI

AI-powered legal information assistant for people living in Spain.

Spain Helper helps users understand immigration, residency and administrative procedures through a reviewed knowledge base and a source-backed legal chat.

The project is built around one core rule:

> Legal answers must come from approved and current materials.  
> When verified information is unavailable, the assistant refuses to guess.

## What it does

- Answers questions about Spanish immigration and administrative procedures
- Searches reviewed legal knowledge
- Shows sources and currentness metadata
- Excludes pending, rejected and outdated materials
- Provides public guides, categories and search
- Includes an administrative legal-review workflow
- Supports controlled RAG reindexing
- Runs locally through Docker Compose

## Legal Core workflow

```text
Source ingestion
      ↓
Legal review task
      ↓
Approval or rejection
      ↓
Approved current knowledge
      ↓
Public legal chat
```

The public chat does not silently fall back to unreviewed RAG content.

When reliable material is missing, the user receives a clear refusal instead of a fabricated legal answer.

## Main features

### Public legal chat

The system:

- detects legal questions;
- retrieves relevant approved knowledge;
- verifies that the material is current;
- generates an answer from the retrieved context;
- returns citations and review metadata.

### Knowledge portal

Users can browse:

- guides;
- legal topics;
- categories;
- searchable public content;
- source information.

### Administration

Authorized administrators can:

- review legal materials;
- approve or reject content;
- inspect knowledge status;
- trigger RAG reindexing;
- manage administrative access;
- view system statistics.

Administrative operations are protected with a bearer token.

## Technology

### Frontend

- React
- TypeScript
- Vite
- TanStack Query
- Tailwind CSS

### Backend

- Rust
- Axum
- Tokio
- SQLx

### Data and AI

- PostgreSQL
- Qdrant
- vector search
- local embeddings
- RAG
- optional local LLM through Ollama

### Infrastructure

- Docker
- Docker Compose

## Architecture

```text
React web application
        │
        ▼
Rust / Axum API
        │
        ├── PostgreSQL
        │   ├── content
        │   ├── metadata
        │   └── legal reviews
        │
        ├── Qdrant
        │   ├── embeddings
        │   └── vector retrieval
        │
        └── Optional local LLM
            └── answer generation
```

## Repository structure

```text
helper/
├── README.md
└── aplicaciones/
    ├── apps/
    │   ├── api/          # Rust backend
    │   └── web/          # React frontend
    ├── docker-compose.yml
    ├── .env.example
    └── README.md         # Detailed technical documentation
```

The application is located in:

```text
aplicaciones/
```

## Quick start

### Requirements

- Git
- Docker Desktop
- Docker Compose

### Clone

```bash
git clone https://github.com/VitalyRuso/helper.git
cd helper/aplicaciones
```

### Configure environment

Linux or macOS:

```bash
cp .env.example .env
```

Windows PowerShell:

```powershell
Copy-Item .env.example .env
```

### Start

```bash
docker compose up --build --wait
```

### Check services

```bash
docker compose ps
```

### View logs

```bash
docker compose logs --tail=200
```

### Stop

```bash
docker compose down
```

Docker volumes are preserved.

## Development

### Backend

```bash
cd aplicaciones/apps/api

cargo fmt --check
cargo check
cargo test
cargo run
```

### Frontend

```bash
cd aplicaciones/apps/web

npm install
npm run dev
```

Production build:

```bash
npm run build
```

## Tests

The backend test suite covers:

- Legal Core integration
- legal-question routing
- approved knowledge retrieval
- rejection of unverified knowledge
- safe refusal behavior
- legal review authorization
- tool adapters

Run:

```bash
cd aplicaciones/apps/api
cargo test
```

## Project status

The current MVP includes:

- working React frontend;
- Rust API;
- PostgreSQL integration;
- Qdrant integration;
- public content and search;
- legal information chat;
- approved-knowledge retrieval;
- citations and currentness metadata;
- legal review workflow;
- protected administrative actions;
- Docker Compose environment;
- integration tests.

## Current limitations

This is a local and portfolio-oriented MVP.

It does not currently provide:

- automatic live monitoring of BOE or Migraciones;
- production user accounts;
- payments;
- cloud production deployment;
- complete coverage of Spanish law;
- guaranteed real-time legal updates.

Ollama is optional, but required for real local LLM generation.

## Disclaimer

Spain Helper provides general informational assistance.

It is not a law firm, does not provide legal representation and is not a substitute for advice from a qualified lawyer or an official Spanish authority.

## Author

Developed by [VitalyRuso](https://github.com/VitalyRuso).
