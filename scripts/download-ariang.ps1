# Download AriaNg standard release and extract to frontend/ directory.
# Usage: .\download-ariang.ps1 [-Version "1.3.8"]

param(
    [string]$Version = "1.3.8"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$FrontendDir = Join-Path $ProjectDir "frontend"

$DownloadUrl = "https://github.com/mayswind/AriaNg/releases/download/$Version/AriaNg-$Version.zip"
$ZipFile = Join-Path $env:TEMP "AriaNg-$Version.zip"

Write-Host "==> Downloading AriaNg v$Version..."
Write-Host "    URL: $DownloadUrl"

# Download
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipFile -UseBasicParsing

# Clean and extract
Write-Host "==> Extracting to $FrontendDir..."
if (Test-Path $FrontendDir) {
    Remove-Item -Recurse -Force $FrontendDir
}
New-Item -ItemType Directory -Force -Path $FrontendDir | Out-Null
Expand-Archive -Path $ZipFile -DestinationPath $FrontendDir -Force

# Verify
$IndexFile = Join-Path $FrontendDir "index.html"
if (-not (Test-Path $IndexFile)) {
    Write-Error "Error: index.html not found after extraction!"
    exit 1
}

# Cleanup
Remove-Item -Force $ZipFile -ErrorAction SilentlyContinue

Write-Host "==> AriaNg v$Version installed successfully to $FrontendDir"
Get-ChildItem $FrontendDir
