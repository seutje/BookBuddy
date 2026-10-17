$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/windows-toolchain.ps1"
Set-Location "$PSScriptRoot/.."
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    $key = Join-Path (Get-Location).Path '.local/release-secrets/bookbuddy.key'
    if (-not (Test-Path -LiteralPath $key)) { throw 'Set TAURI_SIGNING_PRIVATE_KEY to the dedicated BookBuddy key before building.' }
    $env:TAURI_SIGNING_PRIVATE_KEY = $key
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }
& npm.cmd run tauri -- build --ci --bundles nsis -- --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$version = (Get-Content -Raw -LiteralPath package.json | ConvertFrom-Json).version
& node scripts/create-updater-manifest.mjs "v$version"
exit $LASTEXITCODE
