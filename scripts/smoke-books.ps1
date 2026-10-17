$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"

function Set-Text([string]$Label, [string]$Value) {
    $edit = Wait-Element $Label ([System.Windows.Automation.ControlType]::Edit)
    $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Value)
}
function Assert-Text([string]$Label, [string]$Value) {
    $edit = Wait-Element $Label ([System.Windows.Automation.ControlType]::Edit)
    if ($edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value -ne $Value) { throw "Wrong value in $Label" }
}

try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Name-Entry 'Smoke Reader Alpha'
    Click 'Create person'
    Click 'Add book'
    Set-Text 'Title' 'Synthetic shared book'
    Click 'Save book'
    $null = Wait-Element "Smoke Reader Alpha's reading information"
    Set-Text "Smoke Reader Alpha's notes" 'Alpha note stays with Alpha'
    Click "Save Smoke Reader Alpha's reading information"
    Click 'Mark read for Smoke Reader Alpha'
    $null = Wait-Element 'Smoke Reader Alpha: Read · Not set · Not set'
    Assert-Text "Smoke Reader Alpha's notes" 'Alpha note stays with Alpha'
    Click 'Edit shared details'
    Set-Text 'Title' 'Synthetic renamed book'
    Set-Text 'Author(s)' 'Synthetic author'
    Click 'Save shared details'
    $null = Wait-Element 'Synthetic renamed book'
    Click 'Back to Library'
    Set-Text 'Search books' 'SYNTHETIC AUTHOR'
    $null = Wait-Element '1 book'
    Click 'Synthetic renamed book'
    Assert-Text "Smoke Reader Alpha's notes" 'Alpha note stays with Alpha'

    Click 'People'
    Name-Entry 'Smoke Reader Beta'
    Click 'Add person'
    Click 'Library'
    $null = Wait-Element 'No books here yet'
    Select-Person 'Everyone'
    Click 'Synthetic renamed book'
    Select-Person 'Smoke Reader Beta'
    Click "Add to Smoke Reader Beta's library"
    $null = Wait-Element "Save Smoke Reader Beta's reading information"
    Assert-Text "Smoke Reader Beta's notes" ''
    Set-Text "Smoke Reader Beta's notes" 'Beta note stays with Beta'
    Click "Save Smoke Reader Beta's reading information"
    Select-Person 'Smoke Reader Alpha'
    Assert-Text "Smoke Reader Alpha's notes" 'Alpha note stays with Alpha'
    $null = Wait-Element 'Smoke Reader Alpha: Read · Not set · Not set'
    $null = Wait-Element 'Smoke Reader Beta: Not set · Not set · Not set'
    Close-App

    Open-App
    Click 'Synthetic renamed book'
    Assert-Text "Smoke Reader Alpha's notes" 'Alpha note stays with Alpha'
    Select-Person 'Smoke Reader Beta'
    Assert-Text "Smoke Reader Beta's notes" 'Beta note stays with Beta'
    Click 'Back to Library'
    Select-Person 'Everyone'
    $null = Wait-Element '1 book'
    Write-Output 'PASS: offline title-only add, personal notes, Mark read, shared edits, search, separate memberships, context switching and restart through Windows UI and IPC.'
} finally { Close-App }
