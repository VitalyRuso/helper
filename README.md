# Spain Helper AI

Spain Helper AI is an information portal for Russian-speaking newcomers in Spain. The maintained application lives in [`aplicaciones`](aplicaciones/README.md).

It includes public guides and search, an outbound local-LM worker, PostgreSQL, Qdrant, and a reviewed Legal Core. Legal answers use only approved/current material and are informational, not legal advice.

## Start locally

```powershell
cd aplicaciones
Copy-Item .env.example .env
docker compose up --build --wait
```

See:

- [Application and local worker](aplicaciones/README.md)
- [Google Cloud deployment](aplicaciones/deploy/gcp/README.md)

Generated output, credentials, local model files, `.env` files, `target`, `node_modules`, and `dist` must not be committed.
