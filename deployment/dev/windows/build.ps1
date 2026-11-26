$ErrorActionPreference = 'Stop'

$RepoRoot = 'C:\shadowmask'
$ClientSource = "$RepoRoot\clients\flutter"
$ArtifactsDir = "$RepoRoot\deployment\dev\windows\artifacts"
$ToolsDir = 'C:\tools'
$ClientBuild = "$ToolsDir\build\flutter"

function Log($Message) {
    Write-Host "[$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))] [build] $Message"
}

function Import-EnvironmentPath {
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $paths = @($machinePath, $userPath) | Where-Object { $_ }
    $env:PATH = ($paths -join ';').TrimEnd(';')
}

function Find-Winget {
    $candidates = @()
    $cmd = Get-Command winget.exe -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { $candidates += $cmd.Source }
    $candidates += "$env:LOCALAPPDATA\Microsoft\WindowsApps\winget.exe"
    $realExe = Get-ChildItem -Path "$env:ProgramFiles\WindowsApps\Microsoft.DesktopAppInstaller*" -Filter winget.exe -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
    if ($realExe) { $candidates += $realExe }

    foreach ($candidate in ($candidates | Select-Object -Unique)) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            try {
                $null = & $candidate --version
                if ($LASTEXITCODE -eq 0) {
                    return $candidate
                }
            } catch {
                Log "winget at [$candidate] could not be run: [$($_.Exception.Message)]"
            }
        }
    }
    return $null
}

function Get-Winget {
    if (-not (Find-Winget)) {
        Log 'Waiting for winget, which Windows registers at the first logon...'
        Wait-Until -Description 'winget' -Probe { [bool](Find-Winget) } -Minutes 5
    }
    return Find-Winget
}

function Get-WingetArgumentList($WingetId) {
    return @(
        'install', '--id', $WingetId, '--exact', '--source', 'winget',
        '--accept-package-agreements', '--accept-source-agreements',
        '--silent', '--disable-interactivity'
    )
}

function Install-WithWinget($Name, $WingetId, $Probe) {
    Import-EnvironmentPath
    if (& $Probe) {
        Log "$Name already present"
        return
    }
    $wingetPath = Get-Winget
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        Log "Installing [$WingetId] via winget (attempt [$attempt/3])..."
        & $wingetPath (Get-WingetArgumentList $WingetId) | Out-Null
        Import-EnvironmentPath
        if (& $Probe) {
            Log "$Name installed"
            return
        }
        Log "winget exited with code [$LASTEXITCODE]; retrying in 10s..."
        Start-Sleep -Seconds 10
    }
    throw "$Name could not be installed via winget; install it in the box and re-provision"
}

function Get-VsInstallPath {
    foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
        if (Test-Path -LiteralPath "$root\Microsoft Visual Studio\2022\BuildTools") {
            return "$root\Microsoft Visual Studio\2022\BuildTools"
        }
    }
    return $null
}

function Test-VsBuildTool {
    $vsRoot = Get-VsInstallPath
    return [bool]($vsRoot -and (Test-Path -LiteralPath "$vsRoot\VC\Auxiliary\Build\vcvarsall.bat"))
}

function Test-VsArm64Component {
    $vsRoot = Get-VsInstallPath
    if (-not $vsRoot) { return $false }
    return [bool](Get-ChildItem -Path "$vsRoot\MSBuild\Current\Bin\arm64" -Filter MSBuild.exe -ErrorAction SilentlyContinue) -and
        [bool](Get-ChildItem -Path "$vsRoot\VC\Tools\MSVC\*\bin\HostARM64\ARM64\cl.exe" -ErrorAction SilentlyContinue)
}

function Wait-Until($Description, $Probe, $Minutes) {
    $deadline = (Get-Date).AddMinutes($Minutes)
    $checks = 0
    while (-not (& $Probe)) {
        if ((Get-Date) -gt $deadline) {
            throw "$Description was not ready within $Minutes min; install it in the box and re-provision"
        }
        Start-Sleep -Seconds 30
        $checks++
        if ($checks % 20 -eq 0) {
            Log ("...still waiting for {0} (elapsed {1} min)" -f $Description, [int]($checks * 0.5))
        }
    }
}

function Install-VsBuildTool {
    if (Test-VsBuildTool) {
        Log 'Visual Studio Build Tools already present'
    } else {
        $wingetPath = Get-Winget
        Log 'Installing [Microsoft.VisualStudio.2022.BuildTools] via winget (the first run takes 20-40 min)...'
        $arguments = (Get-WingetArgumentList 'Microsoft.VisualStudio.2022.BuildTools') +
            @('--override', '--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
        & $wingetPath $arguments | Out-Null
        Wait-Until -Description 'the Visual Studio Build Tools install' -Probe { Test-VsBuildTool } -Minutes 45
        Log 'Visual Studio Build Tools installed'
    }

    if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -and -not (Test-VsArm64Component)) {
        $setupExe = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\setup.exe"
        if (-not (Test-Path -LiteralPath $setupExe)) {
            throw 'The Visual Studio Installer is missing; add the ARM64 build tools in the box and re-provision'
        }
        Log 'Adding the Visual Studio ARM64 build tools...'
        & $setupExe modify --installPath (Get-VsInstallPath) --norestart --passive `
            --add Microsoft.VisualStudio.Component.VC.Tools.ARM64 Microsoft.VisualStudio.Component.VC.Tools.x86.x64 | Out-Null
        Wait-Until -Description 'the Visual Studio ARM64 build tools' -Probe { Test-VsArm64Component } -Minutes 30
        Log 'Visual Studio ARM64 build tools installed'
    }
}

function Get-FlutterVersion {
    $workflow = Get-Content -Path "$RepoRoot\.github\workflows\build.yml" -Raw
    $match = [regex]::Match($workflow, "flutter-version:\s*'([^']+)'")
    if (-not $match.Success) {
        throw "Could not read flutter-version from [$RepoRoot\.github\workflows\build.yml]"
    }
    return $match.Groups[1].Value
}

function Get-ClientVersion {
    $pubspec = Get-Content -Path "$ClientSource\pubspec.yaml" -Raw
    $match = [regex]::Match($pubspec, '(?m)^version:\s*(\S+)')
    if (-not $match.Success) {
        throw "Could not read the version from [$ClientSource\pubspec.yaml]"
    }
    return $match.Groups[1].Value
}

function Invoke-Checked($Description, [scriptblock]$Command) {
    Log "$Description..."
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code [$LASTEXITCODE]"
    }
}

Install-WithWinget -Name 'Git' -WingetId 'Git.Git' -Probe { [bool](Get-Command git -ErrorAction SilentlyContinue) }
Install-VsBuildTool

$FlutterVersion = Get-FlutterVersion
$FlutterDir = "$ToolsDir\flutter-$FlutterVersion"
$FlutterBat = "$FlutterDir\bin\flutter.bat"
$DartBat = "$FlutterDir\bin\dart.bat"
if (-not (Test-Path -LiteralPath $FlutterBat)) {
    if (Test-Path -LiteralPath $FlutterDir) {
        Log "Removing the incomplete [$FlutterDir]..."
        Remove-Item -Path $FlutterDir -Recurse -Force
    }
    Invoke-Checked "Installing Flutter [$FlutterVersion] into [$FlutterDir]" {
        $null = git clone --single-branch --branch $FlutterVersion https://github.com/flutter/flutter.git $FlutterDir
    }
}
$env:PATH = "$FlutterDir\bin;$env:PATH"

Log "Copying [$ClientSource] to [$ClientBuild]..."
& robocopy $ClientSource $ClientBuild /MIR /IS /A-:H /XJ /XD build .dart_tool ephemeral coverage /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) {
    throw "robocopy failed with exit code [$LASTEXITCODE]"
}

Push-Location $ClientBuild
try {
    Invoke-Checked 'Removing earlier build output (flutter clean)' { & $FlutterBat clean }
    Invoke-Checked 'Fetching packages (flutter pub get)' { & $FlutterBat pub get }
    Invoke-Checked 'Generating sources (build_runner build)' { & $DartBat run build_runner build }
    Invoke-Checked 'Building the client (flutter build windows --release)' { & $FlutterBat build windows --release }
}
finally {
    Pop-Location
}

$Release = Get-Item -Path "$ClientBuild\build\windows\*\runner\Release" -ErrorAction SilentlyContinue |
    Where-Object { $_.PSIsContainer } | Select-Object -First 1
if (-not $Release) {
    throw "flutter build produced no Release output under [$ClientBuild\build\windows]"
}

$Zip = "$ArtifactsDir\shadowmask-client-windows-$(Get-ClientVersion).zip"
New-Item -ItemType Directory -Force -Path $ArtifactsDir | Out-Null
Compress-Archive -Path "$($Release.FullName)\*" -DestinationPath $Zip -Force
Log "Wrote [$Zip]"
