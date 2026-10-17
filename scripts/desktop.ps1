param([ValidateSet('dev', 'build')][string]$Mode = 'dev')
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/windows-toolchain.ps1"
Set-Location "$PSScriptRoot/.."
& npm.cmd run tauri -- $Mode
exit $LASTEXITCODE
