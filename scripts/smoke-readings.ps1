$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$script:application = $null
$today = [DateTime]::Now.ToString('yyyy-MM-dd')
. "$PSScriptRoot/ui-automation.ps1"
try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Name-Entry 'Smoke Reader Alpha'
    Click 'Create person'
    Click 'People'
    Name-Entry 'Smoke Reader Beta'
    Click 'Add person'
    Select-Person 'Smoke Reader Alpha'
    Click 'Library'
    Click 'Add book'
    Set-Text 'Title' 'Repeat reading book'
    Click 'Save book'
    Click 'Mark read for Smoke Reader Alpha'
    $null = Wait-Element 'Household: 0 logged readings · Last reading: Unknown'
    Set-Text "Smoke Reader Alpha's notes" 'Unsaved note survives reading count updates'
    Click 'Log Repeat reading book read today for Smoke Reader Alpha'
    $null = Wait-Element "Household: 1 logged readings · Last reading: $today"
    Assert-Text "Smoke Reader Alpha's notes" 'Unsaved note survives reading count updates'
    Click "Save Smoke Reader Alpha's reading information"
    Click 'Log Repeat reading book read today for Smoke Reader Alpha'
    $null = Wait-Element "Household: 2 logged readings · Last reading: $today"
    Click 'Undo this reading'
    Click 'Confirm undo reading'
    $null = Wait-Element "Household: 1 logged readings · Last reading: $today"
    Click 'Choose date and participants'
    Toggle-Checkbox 'Smoke Reader Beta'
    Set-Text 'Reading note' 'Shared reading note'
    Click 'Save reading'
    $null = Wait-Element "Household: 2 logged readings · Last reading: $today"
    Select-Person 'Smoke Reader Beta'
    $null = Wait-Element "Smoke Reader Beta: 1 logged readings · Last reading: $today"
    Select-Person 'Smoke Reader Alpha'
    Click 'Edit reading 1'
    Set-Text 'Reading note' 'Edited reading note'
    Toggle-Checkbox 'Smoke Reader Beta'
    Click 'Save reading changes'
    $null = Wait-Element 'Edited reading note'
    Select-Person 'Smoke Reader Beta'
    $null = Wait-Element 'Smoke Reader Beta: 0 logged readings · Last reading: Unknown'
    Select-Person 'Smoke Reader Alpha'
    Click 'Delete reading 1'
    Click 'Confirm delete reading'
    $null = Wait-Element "Household: 1 logged readings · Last reading: $today"
    Click 'Activity'
    $null = Wait-Element 'Read again'
    Click 'Log Repeat reading book read today for Smoke Reader Alpha'
    $null = Wait-Element '2 reading events'
    Click 'Undo this reading'
    Click 'Confirm undo reading'
    $null = Wait-Element '1 reading events'
    Close-App
    Open-App
    Click 'Repeat reading book'
    $null = Wait-Element "Smoke Reader Alpha: 1 logged readings · Last reading: $today"
    Assert-Text "Smoke Reader Alpha's notes" 'Unsaved note survives reading count updates'
    Click 'People'
    Click 'Archive Smoke Reader Alpha'
    Click 'Confirm archive'
    Click 'Archive Smoke Reader Beta'
    Click 'Confirm archive'
    Click 'Activity'
    $null = Wait-Element '1 reading events'
    Write-Output 'PASS: manual status without date, repeated quick logs, shared participants, counts, edit/delete/undo, recent shortcuts, draft-note preservation, restart and archived history through offline Windows UI and IPC.'
} finally { Close-App }
