[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "remote-script.ps1")

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
revision_file=/var/lib/spain-helper/previous-revision
if [ ! -s "$revision_file" ]; then
  echo "No previous deployed revision is recorded" >&2
  exit 1
fi
target_revision=$(cat "$revision_file")
case "$target_revision" in
  *[!0-9a-f]*|'')
    echo "Recorded rollback revision is invalid" >&2
    exit 1
    ;;
esac

cd /opt/spain-helper
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "Tracked deployment files have local changes; refusing to overwrite them" >&2
  exit 1
fi
git fetch origin
git cat-file -e "${target_revision}^{commit}"
current_revision=$(git rev-parse HEAD)
git switch --detach "$target_revision"
cd aplicaciones
compose=(docker compose -f docker-compose.yml -f docker-compose.cloud.yml)
"${compose[@]}" config --quiet
"${compose[@]}" up -d --build --wait
"${compose[@]}" ps
printf '%s\n' "$current_revision" >"$revision_file"
'@

try {
    Invoke-GcloudRemoteScript -Gcloud $Gcloud -VmName $VmName -Zone $Zone `
        -Name "rollback" -Script $RemoteScript
} catch {
    throw "Rollback failed. Database migrations are not automatically reversed."
}
