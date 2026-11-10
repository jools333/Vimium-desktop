<#
.SYNOPSIS
Packages the Windows NSIS Installer for Vimium Desktop.
#>
param(
    [string]$Configuration = "release",
    [string]$BinaryPath,
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"

$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } elseif ($MyInvocation.MyCommand.Path) { Split-Path -Parent $MyInvocation.MyCommand.Path } else { "$PWD\scripts" }
$projectRoot = (Resolve-Path "$scriptDir\..").Path

Write-Host "=== Packaging Vimium Desktop Windows Installer ===" -ForegroundColor Cyan

# 1. Locate makensis
$makensis = $null
$possiblePaths = @(
    "C:\Users\lotarev\.w64devkit\bin\makensis.exe",
    "$projectRoot\tools\w64devkit\bin\makensis.exe",
    "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
    "$env:ProgramFiles\NSIS\makensis.exe"
)

foreach ($path in $possiblePaths) {
    if (Test-Path $path) {
        $makensis = $path
        break
    }
}

if (-not $makensis) {
    $cmd = Get-Command "makensis" -ErrorAction SilentlyContinue
    if ($cmd) {
        $makensis = if ($cmd.Source) { $cmd.Source } else { $cmd.Path }
    }
}

if (-not $makensis) {
    Write-Error "NSIS compiler (makensis.exe) was not found. Please install NSIS (e.g. 'winget install NSIS.NSIS')."
    exit 1
}

Write-Host "Using NSIS: $makensis" -ForegroundColor Gray

# 2. Extract version from Cargo.toml
$cargoTomlPath = "$projectRoot\Cargo.toml"
$version = "0.3.1"
if (Test-Path $cargoTomlPath) {
    $content = Get-Content $cargoTomlPath -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        $version = $matches[1]
    }
}
Write-Host "Target Version: $version" -ForegroundColor Gray

# 3. Resolve source executable
if (-not $BinaryPath) {
    $candidate = "$projectRoot\target\$Configuration\vimium-desktop.exe"
    if (Test-Path $candidate) {
        $BinaryPath = $candidate
    } else {
        $msvcCandidate = "$projectRoot\target\x86_64-pc-windows-msvc\$Configuration\vimium-desktop.exe"
        if (Test-Path $msvcCandidate) {
            $BinaryPath = $msvcCandidate
        }
    }
}

if (-not $BinaryPath -or -not (Test-Path $BinaryPath)) {
    Write-Error "Executable not found at '$BinaryPath'. Please compile it first (cargo build --$Configuration)."
    exit 1
}

$resolvedExe = (Resolve-Path $BinaryPath).Path
Write-Host "Source Binary: $resolvedExe" -ForegroundColor Gray

# 4. Prepare output directory
$distDir = "$projectRoot\dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Force -Path $distDir | Out-Null
}

if (-not $OutputPath) {
    $OutputPath = "$distDir\vimium-desktop-windows-x86_64-setup.exe"
}
$resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)

# 5. Ensure icon exists
$iconPath = "$projectRoot\assets\icon.ico"
if (-not (Test-Path $iconPath)) {
    Write-Host "Generating icon assets..." -ForegroundColor Cyan
    Get-Content "$scriptDir\generate-icon.ps1" -Raw | Invoke-Expression
}

# 6. Run makensis
$nsiScript = "$projectRoot\installer\installer.nsi"
Write-Host "Compiling installer: $resolvedOutput" -ForegroundColor Cyan

$nsisArgs = @(
    "/DVERSION=$version",
    "/DSOURCE_EXE=$resolvedExe",
    "/DOUT_FILE=$resolvedOutput",
    $nsiScript
)

& $makensis @nsisArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "makensis failed with exit code $LASTEXITCODE"
    exit $LASTEXITCODE
}

Write-Host "Windows Installer successfully built!" -ForegroundColor Green
Write-Host "Output: $resolvedOutput" -ForegroundColor Green
