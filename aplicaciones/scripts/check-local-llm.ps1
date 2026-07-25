$ErrorActionPreference = "Stop"
$appRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if (-not (Test-Path (Join-Path $appRoot ".env.worker"))) {
    throw "Create aplicaciones/.env.worker from .env.worker.example first."
}

Push-Location $appRoot
try {
    cargo run --release --manifest-path apps/api/Cargo.toml --bin local_worker -- --check
    if ($LASTEXITCODE -ne 0) {
        throw "Local AI check failed with code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}
