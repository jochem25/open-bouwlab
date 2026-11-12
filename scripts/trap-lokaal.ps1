# Trapontwerper lokaal starten (O-2026-10-30-14, O-2026-11-12-01).
#
# Bouwt de frontend (vite) en start de API, die de gebouwde frontend serveert.
# De trapontwerper staat achter inloggen en het recht 'trap'. Lokaal is er geen
# Authentik; daarom start dit script een kleine proxy op localhost
# (scripts\lokaal-auth-proxy.mjs) die een lokale gebruiker met de groep
# 'openbouwlab-trap' meegeeft. Daarna staat de trapontwerper op
# http://localhost:<Poort>/trap. De API zelf luistert op <Poort>+1.
# Database en log komen in target\trap-lokaal\ (niet in de repo).
#
# Gebruik (vanuit de repo-root):
#   powershell -ExecutionPolicy Bypass -File scripts\trap-lokaal.ps1
#   ... -Poort 3011 -GeenBuild -GeenBrowser -ZonderRecht
#   -ZonderRecht: lokale gebruiker zonder het recht 'trap' (toont de melding)
param(
    [int]$Poort = 3011,
    [switch]$GeenBuild,
    [switch]$GeenBrowser,
    [switch]$ZonderRecht
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
$apiPoort = $Poort + 1
$groepen = if ($ZonderRecht) { "lokaal-zonder-recht" } else { "openbouwlab-trap" }
$env:PORT = "$apiPoort"
$env:STATIC_DIR = (Join-Path $root "frontend\dist")
$env:DATABASE_URL = "sqlite://target/trap-lokaal/lokaal.db?mode=rwc"
$url = "http://localhost:$Poort/trap"

$proxy = Start-Process node -ArgumentList @("scripts\lokaal-auth-proxy.mjs", "$Poort", "$apiPoort", $groepen) `
    -NoNewWindow -PassThru
Write-Host "Trapontwerper: $url  (stoppen met Ctrl+C)"
if (-not $GeenBrowser) {
    Start-Job -ScriptBlock { param($u) Start-Sleep -Seconds 3; Start-Process $u } -ArgumentList $url | Out-Null
}
try {
    & (Join-Path $root "target\debug\isso51-api.exe")
} finally {
    if ($proxy -and -not $proxy.HasExited) { Stop-Process -Id $proxy.Id -Force }
}
