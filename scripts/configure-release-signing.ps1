# Requires GitHub CLI authenticated with permission to manage this repository's secrets.
$ErrorActionPreference = 'Stop'
$key = Join-Path (Split-Path $PSScriptRoot -Parent) '.local/release-secrets/bookbuddy.key'
if (-not (Test-Path -LiteralPath $key)) { throw 'The dedicated BookBuddy signing key is missing. Never substitute another application key.' }
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'Install GitHub CLI and run gh auth login, then retry this script.' }
& gh auth status
if ($LASTEXITCODE -ne 0) { throw 'Authenticate GitHub CLI before uploading the signing secret.' }
# Pipe the contents through stdin; never place the private key in arguments or output.
Get-Content -Raw -LiteralPath $key | & gh secret set TAURI_SIGNING_PRIVATE_KEY --repo seutje/BookBuddy
if ($LASTEXITCODE -ne 0) { throw 'GitHub rejected the signing secret.' }
Write-Output 'Configured the dedicated BookBuddy signing secret. Keep the private key backed up securely.'
