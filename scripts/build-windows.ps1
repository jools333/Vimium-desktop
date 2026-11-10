<#
.SYNOPSIS
Builds Vimium Desktop for Windows and creates the NSIS Windows installer.
#>
param(
    [switch]$Debug,
    [switch]$SkipInstaller
)

$ErrorActionPreference = "Stop"

$scriptDir = if ($PSScriptRoot) { $PSScriptRoot } elseif ($MyInvocation.MyCommand.Path) { Split-Path -Parent $MyInvocation.MyCommand.Path } else { "$PWD\scripts" }
$projectRoot = (Resolve-Path "$scriptDir\..").Path

Write-Host "=== Building Vimium Desktop (Windows) ===" -ForegroundColor Cyan

# 0. Configure Toolchain & Incremental
if (Test-Path "C:\Users\lotarev\.w64devkit\bin") {
    $env:PATH = "C:\Users\lotarev\.w64devkit\bin;$env:PATH"
} elseif (Test-Path "$projectRoot\tools\w64devkit\bin") {
    $env:PATH = "$projectRoot\tools\w64devkit\bin;$env:PATH"
}
$env:CARGO_INCREMENTAL = "0"

# 1. Ensure icon exists
$iconPath = "$projectRoot\assets\icon.ico"
if (-not (Test-Path $iconPath)) {
    Write-Host "Generating icon assets..." -ForegroundColor Cyan
    Get-Content "$scriptDir\generate-icon.ps1" -Raw | Invoke-Expression
}

# 2. Build Rust binary
Write-Host "Building Rust binary..." -ForegroundColor Cyan
$config = if ($Debug) { "debug" } else { "release" }

if ($Debug) {
    cargo build
} else {
    cargo build --release
}

Write-Host "Build finished successfully!" -ForegroundColor Green
Write-Host "Artifact: target\$config\vimium-desktop.exe" -ForegroundColor Green

# 3. Build Windows NSIS Installer
if (-not $SkipInstaller) {
    Write-Host "Packaging Windows Installer..." -ForegroundColor Cyan
    $installerScript = "$scriptDir\build-installer.ps1"
    $sb = [scriptblock]::Create((Get-Content $installerScript -Raw))
    & $sb -Configuration $config
}
