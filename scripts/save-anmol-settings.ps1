# Snapshot the active OpenHuman user config (no secrets — keys stay in the OS store).
# Usage: powershell -File scripts/save-anmol-settings.ps1

$ErrorActionPreference = "Stop"
$usersRoot = Join-Path $env:USERPROFILE ".openhuman\users"
$snapDir = Join-Path $env:USERPROFILE ".openhuman\anmol-settings"
$snapFile = Join-Path $snapDir "config.toml"

$live = Get-ChildItem $usersRoot -Directory |
    ForEach-Object {
        $cfg = Join-Path $_.FullName "config.toml"
        if (Test-Path $cfg) { Get-Item $cfg }
    } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if (-not $live) { throw "No OpenHuman user config.toml found under $usersRoot" }

New-Item -ItemType Directory -Force -Path $snapDir | Out-Null
Copy-Item $live.FullName $snapFile -Force
Write-Host "Saved $($live.FullName) -> $snapFile"
Write-Host "mtime $($live.LastWriteTime)"
