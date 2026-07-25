[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectNumber = "745947699440"
$Zone = "europe-west1-b"
$VmName = "spain-helper"

$GcloudCommand = Get-Command gcloud.cmd, gcloud -ErrorAction SilentlyContinue |
    Select-Object -First 1
$Gcloud = if ($GcloudCommand) {
    $GcloudCommand.Source
} else {
    Join-Path $env:LOCALAPPDATA "Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"
}
if (-not (Test-Path -LiteralPath $Gcloud)) {
    throw "Google Cloud CLI is not installed."
}

$SdkRoot = Split-Path (Split-Path $Gcloud -Parent) -Parent
$BundledPython = Join-Path $SdkRoot "platform\bundledpython\python.exe"
if (Test-Path -LiteralPath $BundledPython) {
    $env:CLOUDSDK_PYTHON = $BundledPython
} elseif ($env:CLOUDSDK_PYTHON -and -not (Test-Path -LiteralPath $env:CLOUDSDK_PYTHON)) {
    Remove-Item Env:CLOUDSDK_PYTHON
}

$ProjectId = (& $Gcloud projects describe $ProjectNumber "--format=value(projectId)").Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($ProjectId)) {
    throw "Could not resolve Google Cloud project number $ProjectNumber."
}
& $Gcloud config set project $ProjectId
if ($LASTEXITCODE -ne 0) {
    throw "Could not select Google Cloud project '$ProjectId'."
}

$RemoteScript = @'
set -euo pipefail
cd /opt/spain-helper
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "Tracked deployment files have local changes; refusing to overwrite them" >&2
  exit 1
fi
git switch main
previous_revision=$(git rev-parse HEAD)
git pull --ff-only origin main
current_revision=$(git rev-parse HEAD)
if [ "$previous_revision" != "$current_revision" ]; then
  install -d -m 700 /var/lib/spain-helper
  printf '%s\n' "$previous_revision" >/var/lib/spain-helper/previous-revision
fi
cd aplicaciones
compose=(docker compose -f docker-compose.yml -f docker-compose.cloud.yml)
"${compose[@]}" config --quiet
"${compose[@]}" up -d --build --wait
"${compose[@]}" ps
'@

$RemoteScript | & $Gcloud compute ssh $VmName `
    --zone=$Zone --tunnel-through-iap --quiet --command="sudo bash -s"
if ($LASTEXITCODE -ne 0) {
    throw "Update failed. Run the rollback script only if a previous revision was recorded."
}
