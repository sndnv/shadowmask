$ErrorActionPreference = 'Stop'

$WindowsDir = 'C:\shadowmask\deployment\dev\windows'
$ArtifactsDir = "$WindowsDir\artifacts"
$InstallDir = "$env:LOCALAPPDATA\Programs\shadowmask"
$ServerPort = if ($env:SHADOWMASK_PORT) { [int]$env:SHADOWMASK_PORT } else { 8080 }
$ServerUrl = "http://10.0.2.2:$ServerPort"
$CiPolicyKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy'
$SmartAppControlPolicyId = '0283ac0f-fff1-49ae-ada1-8a933130cad6'

function Log($Message) {
    Write-Host "[$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))] [provision] $Message"
}

function Test-ShadowmaskServer($Url) {
    try {
        $health = Invoke-RestMethod -Uri "$Url/health" -UseBasicParsing -TimeoutSec 5
        return ($health.status -eq 'ok')
    } catch {
        return $false
    }
}

function Test-SmartAppControlEnforced {
    $Policies = (& CiTool.exe --list-policies --json | Out-String | ConvertFrom-Json).Policies
    return [bool]($Policies | Where-Object { $_.PolicyID -eq $SmartAppControlPolicyId -and $_.IsEnforced })
}

Log "Checking that the server answers at [$ServerUrl/health]..."
if (-not (Test-ShadowmaskServer $ServerUrl)) {
    throw "No Shadowmask server answers at [$ServerUrl/health]. Start the dev stack on the host (from [deployment/dev]: 'docker compose up --build' or 'podman compose up --build'), set SHADOWMASK_PORT if it is not on 8080, and re-run 'vagrant provision'."
}

Log 'Checking Smart App Control...'
$SmartAppControlState = (Get-ItemProperty -Path $CiPolicyKey -ErrorAction SilentlyContinue).VerifiedAndReputablePolicyState
if ($null -eq $SmartAppControlState) {
    Log 'Smart App Control is not available on this system'
} else {
    if ($SmartAppControlState -ne 0) {
        Log "Turning Smart App Control off (state [$SmartAppControlState])..."
        Set-ItemProperty -Path $CiPolicyKey -Name 'VerifiedAndReputablePolicyState' -Value 0 -Type DWord
        & CiTool.exe --refresh --json | Out-Null
    }
    if (Test-SmartAppControlEnforced) {
        throw "Smart App Control is turned off but still enforced until the VM restarts; run 'vagrant reload --provision' with the same SHADOWMASK_* variables."
    }
    Log 'Smart App Control is off'
}

if ($env:SHADOWMASK_WIN_ARTIFACTS -eq 'build') {
    Log 'Building the client in the guest (SHADOWMASK_WIN_ARTIFACTS=build)...'
    & "$WindowsDir\build.ps1"
}

$Zip = Get-ChildItem -Path $ArtifactsDir -Filter 'shadowmask-client-windows-*.zip' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $Zip) {
    throw "No [shadowmask-client-windows-*.zip] under [$ArtifactsDir]; see artifacts\README.md"
}

$Running = @(Get-Process -Name 'shadowmask' -ErrorAction SilentlyContinue)
foreach ($Process in $Running) {
    Log "Stopping [shadowmask] with PID [$($Process.Id)]..."
    Stop-Process -InputObject $Process -Force
    if (-not $Process.WaitForExit(30000)) {
        throw "[shadowmask] with PID [$($Process.Id)] is still running after 30s"
    }
}

Log "Installing [$($Zip.Name)] into [$InstallDir]..."
if (Test-Path -LiteralPath $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force
}
Expand-Archive -Path $Zip.FullName -DestinationPath $InstallDir
$Exe = "$InstallDir\shadowmask.exe"
if (-not (Test-Path -LiteralPath $Exe)) {
    throw "[$($Zip.Name)] has no [shadowmask.exe] at its top level"
}

$Shell = New-Object -ComObject WScript.Shell
foreach ($Folder in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'))) {
    $Shortcut = $Shell.CreateShortcut("$Folder\Shadowmask.lnk")
    $Shortcut.TargetPath = $Exe
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.Save()
}

Log "Provisioning complete. Open Shadowmask from the Start menu and use the server [$ServerUrl]."
