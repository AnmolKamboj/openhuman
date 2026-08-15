# Restore the Anmol OpenHuman snapshot into the active user dir.
# Safe after a config wipe, a new Google login (new users/<id>/), or a parse reset.
# Usage: quit OpenHuman first, then: powershell -File scripts/restore-anmol-settings.ps1

$ErrorActionPreference = "Stop"
$usersRoot = Join-Path $env:USERPROFILE ".openhuman\users"
$snapFile = Join-Path $env:USERPROFILE ".openhuman\anmol-settings\config.toml"

if (-not (Test-Path $snapFile)) { throw "No snapshot at $snapFile — run save-anmol-settings.ps1 first" }

$running = Get-Process -Name OpenHuman -ErrorAction SilentlyContinue
if ($running) {
    throw "OpenHuman is running (pid $($running.Id -join ',')). Quit it first or this restore will be overwritten."
}

$targets = Get-ChildItem $usersRoot -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "config.toml") }

if (-not $targets) { throw "No user config.toml under $usersRoot" }

foreach ($dir in $targets) {
    $cfg = Join-Path $dir.FullName "config.toml"
    $bak = Join-Path $dir.FullName ("config.toml.bak-before-restore-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
    Copy-Item $cfg $bak -Force
    Copy-Item $snapFile $cfg -Force
    Write-Host "Restored -> $cfg (backup $bak)"
}

Write-Host "Done. Start scripts/run-dev-win.sh. Do not click through onboarding."
