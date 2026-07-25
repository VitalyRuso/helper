[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "remote-script.ps1")

$ProjectNumber = "745947699440"
$Region = "europe-west1"
$Zone = "europe-west1-b"
$Network = "spain-helper-net"
$Subnet = "spain-helper-eu"
$AddressName = "spain-helper-ip"
$VmName = "spain-helper"
$Repository = "https://github.com/VitalyRuso/helper.git"

function Get-GcloudPath {
    $command = Get-Command gcloud.cmd, gcloud -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) {
        return $command.Source
    }

    $installed = Join-Path $env:LOCALAPPDATA "Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"
    if (Test-Path -LiteralPath $installed) {
        return $installed
    }

    throw "Google Cloud CLI is not installed. Install it and run 'gcloud auth login'."
}

$Gcloud = Get-GcloudPath
$SdkRoot = Split-Path (Split-Path $Gcloud -Parent) -Parent
$BundledPython = Join-Path $SdkRoot "platform\bundledpython\python.exe"
if (Test-Path -LiteralPath $BundledPython) {
    $env:CLOUDSDK_PYTHON = $BundledPython
} elseif ($env:CLOUDSDK_PYTHON -and -not (Test-Path -LiteralPath $env:CLOUDSDK_PYTHON)) {
    Remove-Item Env:CLOUDSDK_PYTHON
}

function Invoke-Gcloud([string[]]$Arguments) {
    & $Gcloud @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "gcloud failed: $($Arguments -join ' ')"
    }
}

function Test-GcloudResource([string[]]$Arguments) {
    $PreviousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $Gcloud @Arguments 1>$null 2>$null
        return $LASTEXITCODE -eq 0
    } finally {
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
}

$ActiveAccounts = & $Gcloud auth list --filter=status:ACTIVE "--format=value(account)"
$AuthExitCode = $LASTEXITCODE
$ActiveAccount = $ActiveAccounts | Select-Object -First 1
if ($AuthExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($ActiveAccount)) {
    throw "No active Google Cloud account. Run 'gcloud auth login'."
}

$ProjectId = (& $Gcloud projects describe $ProjectNumber "--format=value(projectId)").Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($ProjectId)) {
    throw "Could not resolve Google Cloud project number $ProjectNumber."
}

Invoke-Gcloud @("config", "set", "project", $ProjectId)
Invoke-Gcloud @("config", "set", "compute/region", $Region)
Invoke-Gcloud @("config", "set", "compute/zone", $Zone)
Invoke-Gcloud @("services", "enable", "compute.googleapis.com", "iap.googleapis.com")

if (-not (Test-GcloudResource @("compute", "networks", "describe", $Network))) {
    Invoke-Gcloud @("compute", "networks", "create", $Network, "--subnet-mode=custom")
}

if (-not (Test-GcloudResource @(
            "compute", "networks", "subnets", "describe", $Subnet, "--region=$Region"
        ))) {
    Invoke-Gcloud @(
        "compute", "networks", "subnets", "create", $Subnet,
        "--network=$Network", "--region=$Region", "--range=10.20.0.0/24"
    )
}

if (-not (Test-GcloudResource @(
            "compute", "addresses", "describe", $AddressName, "--region=$Region"
        ))) {
    Invoke-Gcloud @("compute", "addresses", "create", $AddressName, "--region=$Region")
}

$StaticIp = (& $Gcloud compute addresses describe $AddressName --region=$Region "--format=value(address)").Trim()
if ($LASTEXITCODE -ne 0 -or $StaticIp -notmatch '^\d{1,3}(\.\d{1,3}){3}$') {
    throw "The reserved address '$AddressName' did not return a valid IPv4 address."
}

$WebRule = "spain-helper-allow-web"
if (Test-GcloudResource @("compute", "firewall-rules", "describe", $WebRule)) {
    Invoke-Gcloud @(
        "compute", "firewall-rules", "update", $WebRule,
        "--rules=tcp:80,tcp:443", "--source-ranges=0.0.0.0/0",
        "--target-tags=spain-helper-web"
    )
} else {
    Invoke-Gcloud @(
        "compute", "firewall-rules", "create", $WebRule,
        "--network=$Network", "--direction=INGRESS", "--action=ALLOW",
        "--rules=tcp:80,tcp:443", "--source-ranges=0.0.0.0/0",
        "--target-tags=spain-helper-web"
    )
}

$SshRule = "spain-helper-allow-iap-ssh"
if (Test-GcloudResource @("compute", "firewall-rules", "describe", $SshRule)) {
    Invoke-Gcloud @(
        "compute", "firewall-rules", "update", $SshRule,
        "--rules=tcp:22", "--source-ranges=35.235.240.0/20",
        "--target-tags=spain-helper-iap"
    )
} else {
    Invoke-Gcloud @(
        "compute", "firewall-rules", "create", $SshRule,
        "--network=$Network", "--direction=INGRESS", "--action=ALLOW",
        "--rules=tcp:22", "--source-ranges=35.235.240.0/20",
        "--target-tags=spain-helper-iap"
    )
}

$VmExists = Test-GcloudResource @("compute", "instances", "describe", $VmName, "--zone=$Zone")
if (-not $VmExists) {
    Invoke-Gcloud @(
        "compute", "instances", "create", $VmName,
        "--zone=$Zone", "--machine-type=e2-standard-2", "--subnet=$Subnet",
        "--address=$StaticIp", "--tags=spain-helper-web,spain-helper-iap",
        "--boot-disk-size=30GB", "--boot-disk-type=pd-balanced",
        "--image-family=debian-12", "--image-project=debian-cloud",
        "--restart-on-failure", "--no-service-account", "--no-scopes",
        "--shielded-secure-boot", "--shielded-vtpm",
        "--shielded-integrity-monitoring", "--metadata=enable-oslogin=TRUE"
    )
} else {
    $Vm = & $Gcloud compute instances describe $VmName --zone=$Zone --format=json |
        ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect VM '$VmName'."
    }
    $VmIp = $Vm.networkInterfaces[0].accessConfigs[0].natIP
    $MachineType = ($Vm.machineType -split "/")[-1]
    if ($VmIp -ne $StaticIp) {
        throw "VM '$VmName' uses $VmIp, but reserved address '$AddressName' is $StaticIp."
    }
    if ($MachineType -ne "e2-standard-2") {
        throw "VM '$VmName' uses $MachineType; expected e2-standard-2."
    }
    $BootDisk = (($Vm.disks | Where-Object boot).source -split "/")[-1]
    $BootDiskSize = [int](& $Gcloud compute disks describe $BootDisk `
            --zone=$Zone "--format=value(sizeGb)")
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect boot disk '$BootDisk'."
    }
    if ($BootDiskSize -lt 30) {
        Invoke-Gcloud @(
            "compute", "disks", "resize", $BootDisk, "--zone=$Zone",
            "--size=30GB", "--quiet"
        )
    }
    Invoke-Gcloud @(
        "compute", "instances", "add-tags", $VmName, "--zone=$Zone",
        "--tags=spain-helper-web,spain-helper-iap"
    )
    Invoke-Gcloud @(
        "compute", "instances", "add-metadata", $VmName, "--zone=$Zone",
        "--metadata=enable-oslogin=TRUE"
    )
    Invoke-Gcloud @(
        "compute", "instances", "set-scheduling", $VmName, "--zone=$Zone",
        "--restart-on-failure"
    )
    if ($Vm.status -eq "TERMINATED") {
        Invoke-Gcloud @("compute", "instances", "start", $VmName, "--zone=$Zone")
    }
}

$SshArguments = @(
    "compute", "ssh", $VmName, "--zone=$Zone", "--tunnel-through-iap", "--quiet"
)
$SshReady = $false
for ($Attempt = 1; $Attempt -le 18; $Attempt++) {
    $PreviousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $Gcloud @SshArguments --command=true 1>$null 2>$null
        $SshExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
    if ($SshExitCode -eq 0) {
        $SshReady = $true
        break
    }
    Start-Sleep -Seconds 10
}
if (-not $SshReady) {
    throw "VM '$VmName' did not become reachable through IAP SSH."
}

$PublicHost = "spain-helper-$($StaticIp -replace '\.', '-').sslip.io"
$RemoteTemplate = @'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y ca-certificates curl git openssl
if ! command -v docker >/dev/null || ! docker compose version >/dev/null 2>&1; then
  install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
  chmod a+r /etc/apt/keyrings/docker.asc
  . /etc/os-release
  printf '%s\n' \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian ${VERSION_CODENAME} stable" \
    >/etc/apt/sources.list.d/docker.list
  apt-get update
  apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
fi
systemctl enable --now docker

install -d -m 700 /var/lib/spain-helper
if [ ! -d /opt/spain-helper/.git ]; then
  if [ -e /opt/spain-helper ]; then
    echo "/opt/spain-helper exists but is not a Git checkout" >&2
    exit 1
  fi
  git clone --branch main --single-branch __REPOSITORY__ /opt/spain-helper
else
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
    printf '%s\n' "$previous_revision" >/var/lib/spain-helper/previous-revision
  fi
fi

cd /opt/spain-helper/aplicaciones
umask 077
touch .env
chmod 600 .env

set_env() {
  key="$1"
  value="$2"
  if grep -q "^${key}=" .env; then
    sed -i "s|^${key}=.*|${key}=${value}|" .env
  else
    printf '%s=%s\n' "$key" "$value" >>.env
  fi
}

ensure_secret() {
  key="$1"
  value=$(sed -n "s/^${key}=//p" .env | head -n 1)
  if [ -z "$value" ]; then
    set_env "$key" "$(openssl rand -hex 32)"
  fi
}

set_env APP_ENV production
set_env API_HOST 0.0.0.0
set_env API_PORT 8000
set_env WEB_PORT 3000
set_env PUBLIC_HOST __PUBLIC_HOST__
set_env VITE_API_URL https://__PUBLIC_HOST__
set_env POSTGRES_DB spain_helper
set_env POSTGRES_USER postgres
ensure_secret POSTGRES_PASSWORD
postgres_password=$(sed -n 's/^POSTGRES_PASSWORD=//p' .env | head -n 1)
set_env DATABASE_URL "postgres://postgres:${postgres_password}@postgres:5432/spain_helper"
set_env QDRANT_URL http://qdrant:6333
set_env QDRANT_COLLECTION spain_helper_main
set_env DOCS_DIR /app/docs
set_env CHUNK_SIZE 1200
set_env CHUNK_OVERLAP 200
set_env TOP_K 5
set_env EMBEDDING_PROVIDER fastembed
set_env EMBEDDING_MODEL intfloat/multilingual-e5-small
set_env EMBEDDING_FALLBACK disabled
ensure_secret ACCESS_KEYS
set_env GUEST_QUESTION_LIMIT 3
set_env ADMIN_USERNAME admin
ensure_secret ADMIN_PASSWORD
ensure_secret ADMIN_TOKEN
ensure_secret LOCAL_WORKER_TOKEN
set_env CORS_ORIGINS https://__PUBLIC_HOST__
set_env LOG_LEVEL info

compose=(docker compose -f docker-compose.yml -f docker-compose.cloud.yml)
"${compose[@]}" config --quiet
"${compose[@]}" up -d --build --wait
"${compose[@]}" ps
'@

$RemoteScript = $RemoteTemplate.
    Replace("__REPOSITORY__", $Repository).
    Replace("__PUBLIC_HOST__", $PublicHost)
try {
    Invoke-GcloudRemoteScript -Gcloud $Gcloud -VmName $VmName -Zone $Zone `
        -Name "deploy" -Script $RemoteScript
} catch {
    throw "Remote deployment failed. Infrastructure was left in place for diagnosis."
}

$HealthUrl = "https://$PublicHost/health"
$LastTlsError = ""
$Healthy = $false
for ($Attempt = 1; $Attempt -le 24; $Attempt++) {
    try {
        $Response = Invoke-WebRequest -Uri $HealthUrl -UseBasicParsing -TimeoutSec 10
        if ($Response.StatusCode -eq 200) {
            $Healthy = $true
            break
        }
    } catch {
        $LastTlsError = $_.Exception.Message
    }
    Start-Sleep -Seconds 5
}
if (-not $Healthy) {
    throw "HTTPS validation failed for $HealthUrl. TLS verification was not bypassed. Last error: $LastTlsError"
}

$WorkerTokenExitCode = 1
$PreviousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $WorkerTokenOutput = & $Gcloud @SshArguments `
        --command="sudo sed -n 's/^LOCAL_WORKER_TOKEN=//p' /opt/spain-helper/aplicaciones/.env"
    $WorkerTokenExitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $PreviousErrorActionPreference
}
if ($WorkerTokenExitCode -ne 0) {
    throw "Deployment is healthy, but LOCAL_WORKER_TOKEN could not be read through IAP SSH."
}
$WorkerToken = @($WorkerTokenOutput) |
    ForEach-Object { $_.Trim() } |
    Where-Object { $_ -match '^[0-9a-f]{64}$' } |
    Select-Object -Last 1
if ([string]::IsNullOrWhiteSpace($WorkerToken)) {
    throw "Deployment is healthy, but the server returned no valid LOCAL_WORKER_TOKEN."
}

$ApplicationRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$WorkerEnvPath = Join-Path $ApplicationRoot ".env.worker"
$WorkerId = if ([string]::IsNullOrWhiteSpace($env:COMPUTERNAME)) {
    "windows-worker"
} else {
    $env:COMPUTERNAME.ToLowerInvariant()
}
$WorkerEnv = @"
SPAIN_HELPER_API_URL=https://$PublicHost
LOCAL_WORKER_TOKEN=$WorkerToken
LOCAL_WORKER_ID=$WorkerId
LM_STUDIO_BASE_URL=http://127.0.0.1:1234/v1
LM_STUDIO_PREFERRED_MODEL=Qwen3.6-27B-GGUF
"@
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($WorkerEnvPath, $WorkerEnv, $Utf8NoBom)
$Identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
& icacls.exe $WorkerEnvPath /inheritance:r /grant:r "${Identity}:(F)" 1>$null
if ($LASTEXITCODE -ne 0) {
    Remove-Item -LiteralPath $WorkerEnvPath -Force
    throw "Deployment is healthy, but a private ACL could not be applied to .env.worker."
}

Write-Output "Project ID: $ProjectId"
Write-Output "VM: $VmName ($Zone, e2-standard-2)"
Write-Output "Static IP: $StaticIp"
Write-Output "HTTPS URL: https://$PublicHost"
Write-Output "Worker config: $WorkerEnvPath"
Write-Output "Cloud secrets remain in /opt/spain-helper/aplicaciones/.env; only the worker token was copied locally."
