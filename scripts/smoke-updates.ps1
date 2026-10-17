$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
. "$PSScriptRoot/ui-automation.ps1"
try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Name-Entry 'Synthetic updater reader'
    Click 'Create person'
    Click 'Settings'
    Click 'Check for updates'
    $null = Wait-Element 'Update checks are available in installed release builds.'
    $null = Wait-Element 'Export backup'
    Click 'Library'
    $null = Wait-Element 'Add book'
    Write-Output 'PASS: isolated release update checks stay disabled; manual status is accessible and the offline library remains usable.'
} finally { Close-App }
