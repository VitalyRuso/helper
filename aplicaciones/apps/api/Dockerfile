FROM rust:1-trixie AS build
WORKDIR /workspace
COPY tools ./tools
COPY aplicaciones/apps/api ./aplicaciones/apps/api
RUN cargo build --release --manifest-path aplicaciones/apps/api/Cargo.toml

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /workspace/aplicaciones/apps/api/target/release/spain-helper-api /usr/local/bin/spain-helper-api
COPY --from=build /workspace/aplicaciones/apps/api/target/release/index_docs /usr/local/bin/index_docs

EXPOSE 8000

CMD ["spain-helper-api"]
