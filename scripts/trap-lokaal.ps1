# Trapontwerper lokaal starten (O-2026-10-30-14).
#
# Bouwt de frontend (vite) en start de API, die de gebouwde frontend serveert.
# Daarna staat de trapontwerper op http://localhost:<Poort>/trap.
# Database en log komen in target\trap-lokaal\ (niet in de repo).
#
# Gebruik (vanuit de repo-root):
#   powershell -ExecutionPolicy Bypass -File scripts\trap-lokaal.ps1
#   ... -Poort 3011 -GeenBuild -GeenBrowser
param(
    [int]$Poort = 3011,
    [switch]$GeenBuild,
    [switch]$GeenBrowser
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $GeenBuild) {
    Push-Location frontend
    if (-not (Test-Path node_modules)) { npm ci; if ($LASTEXITCODE -ne 0) { throw "npm ci mislukt" } }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "frontend-build mislukt" }
    Pop-Location
    cargo build -p isso51-api
    if ($LASTEXITCODE -ne 0) { throw "API-build mislukt" }
}

$uit = Join-Path $root "target\trap-lokaal"
New-Item -ItemType Directory -Force $uit | Out-Null
$env:PORT = "$Poort"
$env:STATIC_DIR = (Join-Path $root "frontend\dist")
$env:DATABASE_URL = "sqlite://target/trap-lokaal/lokaal.db?mode=rwc"
$url = "http://localhost:$Poort/trap"
Write-Host "Trapontwerper: $url  (stoppen met Ctrl+C)"
if (-not $GeenBrowser) {
    Start-Job -ScriptBlock { param($u) Start-Sleep -Seconds 3; Start-Process $u } -ArgumentList $url | Out-Null
}
& (Join-Path $root "target\debug\isso51-api.exe")
