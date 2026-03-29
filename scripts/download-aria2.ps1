# Download aria2c binary for Windows and place it in src-tauri/binaries/.
# Usage: .\download-aria2.ps1 [-Target "x86_64-pc-windows-msvc"] [-Aria2Version "1.37.0"]

param(
    [string]$Target = "",
    [string]$Aria2Version = "1.37.0"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$BinariesDir = Join-Path $ProjectDir "src-tauri\binaries"

# Auto-detect target if not specified
if (-not $Target) {
    $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($Arch) {
        "X64"   { $Target = "x86_64-pc-windows-msvc" }
        "Arm64" { $Target = "aarch64-pc-windows-msvc" }
        default { Write-Error "Unsupported architecture: $Arch"; exit 1 }
    }
}

$OutputFile = Join-Path $BinariesDir "aria2c-$Target.exe"

Write-Host "==> Downloading aria2 v$Aria2Version for $Target..."

# Create binaries directory
New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null

# Determine download URL based on architecture
$ArchSuffix = if ($Target -like "*aarch64*") { "arm64" } else { "64bit" }
$DownloadUrl = "https://github.com/aria2/aria2/releases/download/release-$Aria2Version/aria2-$Aria2Version-win-$ArchSuffix-build1.zip"

Write-Host "    URL: $DownloadUrl"

$ZipFile = Join-Path $env:TEMP "aria2-$Aria2Version-win.zip"
$ExtractDir = Join-Path $env:TEMP "aria2-extract"

# Download
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipFile -UseBasicParsing

# Extract
if (Test-Path $ExtractDir) {
    Remove-Item -Recurse -Force $ExtractDir
}
Expand-Archive -Path $ZipFile -DestinationPath $ExtractDir -Force

# Find aria2c.exe in extracted files
$Aria2Exe = Get-ChildItem -Path $ExtractDir -Recurse -Filter "aria2c.exe" | Select-Object -First 1

if (-not $Aria2Exe) {
    Write-Error "Error: aria2c.exe not found in downloaded archive!"
    exit 1
}

# Copy to target location
Copy-Item -Path $Aria2Exe.FullName -Destination $OutputFile -Force

# Cleanup
Remove-Item -Force $ZipFile -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $ExtractDir -ErrorAction SilentlyContinue

Write-Host "==> aria2c binary placed at: $OutputFile"
Write-Host "    Size: $((Get-Item $OutputFile).Length / 1MB) MB"
