param([switch]$SkipKeyboard)
$ErrorActionPreference = 'Stop'
foreach ($test in @('people','books','fields','readings','import-preview','import-commit','backups','keyboard','pagination')) {
    if ($SkipKeyboard -and $test -eq 'keyboard') { Write-Output 'Keyboard rerun excluded explicitly; run smoke-keyboard.ps1 with the test window in the foreground.'; continue }
    Write-Output "Running offline Windows acceptance: $test"
    & powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/smoke-$test.ps1"
    if ($LASTEXITCODE -ne 0) { throw "Offline acceptance failed: $test" }
}
Write-Output 'PASS: complete offline Windows desktop acceptance suite.'
