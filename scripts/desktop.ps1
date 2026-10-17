param([ValidateSet('dev', 'build')][string]$Mode = 'dev')
$ErrorActionPreference = 'Stop'
if ($Mode -eq 'build') {
    & powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/release-build.ps1"
    exit $LASTEXITCODE
}
. "$PSScriptRoot/windows-toolchain.ps1"
Set-Location "$PSScriptRoot/.."
& npm.cmd run tauri -- $Mode
exit $LASTEXITCODE
