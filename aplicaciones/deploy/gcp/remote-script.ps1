function Invoke-GcloudRemoteScript {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Gcloud,
        [Parameter(Mandatory)]
        [string]$VmName,
        [Parameter(Mandatory)]
        [string]$Zone,
        [Parameter(Mandatory)]
        [string]$Name,
        [Parameter(Mandatory)]
        [string]$Script
    )

    if ($Name -notmatch '^[a-z0-9-]+$') {
        throw "Remote script name '$Name' is invalid."
    }

    $Suffix = [Guid]::NewGuid().ToString("N")
    $RemotePath = "/tmp/spain-helper-$Name-$Suffix.sh"
    $LocalPath = Join-Path ([IO.Path]::GetTempPath()) "spain-helper-$Name-$Suffix.sh"
    $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

    try {
        [IO.File]::WriteAllText($LocalPath, ($Script -replace "`r`n", "`n"), $Utf8NoBom)

        $CopyExitCode = 1
        $PreviousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            & $Gcloud compute scp $LocalPath "${VmName}:$RemotePath" `
                --zone=$Zone --tunnel-through-iap --quiet
            $CopyExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $PreviousErrorActionPreference
        }
        if ($CopyExitCode -ne 0) {
            throw "Could not copy the remote $Name script through IAP."
        }

        $RemoteCommand = "sudo bash '$RemotePath'; rc=`$?; rm -f '$RemotePath'; exit `$rc"
        $RunExitCode = 1
        $PreviousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            & $Gcloud compute ssh $VmName --zone=$Zone --tunnel-through-iap `
                --quiet "--command=$RemoteCommand"
            $RunExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $PreviousErrorActionPreference
        }
        if ($RunExitCode -ne 0) {
            throw "Remote $Name script failed."
        }
    } finally {
        if (Test-Path -LiteralPath $LocalPath) {
            Remove-Item -LiteralPath $LocalPath -Force
        }
    }
}
