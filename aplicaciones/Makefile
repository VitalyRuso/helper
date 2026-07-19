.PHONY: dev api web db index seed test docker-build docker-up docker-down

dev:
	docker compose up postgres qdrant

api:
	cd apps/api && cargo run

web:
	cd apps/web && npm run dev -- --host 0.0.0.0

db:
	docker compose up postgres qdrant

index:
	cd apps/api && cargo run --bin index_docs

seed:
	cd apps/api && cargo run

test:
	cd apps/api && cargo test && cd ../web && npm run build

docker-build:
	docker compose build

docker-up:
	docker compose up

docker-down:
	docker compose down
