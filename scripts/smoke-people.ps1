# Exercises real IPC and UI on a fresh, isolated debug-only database.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$script:application = $null

. "$PSScriptRoot/ui-automation.ps1"

try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Name-Entry 'Smoke Reader Alpha'
    Click 'Create person'
    $null = Wait-Element "Viewing Smoke Reader Alpha’s library"
    Click 'People'
    Name-Entry 'Smoke Reader Beta'
    Click 'Add person'
    $null = Wait-Element "Viewing Smoke Reader Beta’s library"
    Select-Person 'Smoke Reader Alpha'
    $null = Wait-Element "Viewing Smoke Reader Alpha’s library"
    Click 'Rename Smoke Reader Alpha'
    Name-Entry 'Smoke Reader Renamed'
    Click 'Save name'
    $null = Wait-Element "Viewing Smoke Reader Renamed’s library"
    Close-App

    Open-App
    $null = Wait-Element "Viewing Smoke Reader Renamed’s library"
    Click 'People'
    Click 'Archive Smoke Reader Renamed'
    Click 'Confirm archive'
    $null = Wait-Element 'Viewing everyone in the household'
    $null = Wait-Element 'Smoke Reader RenamedArchived · history preserved' ([System.Windows.Automation.ControlType]::ListItem)
    Select-Person 'Smoke Reader Beta'
    $null = Wait-Element "Viewing Smoke Reader Beta’s library"
    Close-App

    Open-App
    $null = Wait-Element "Viewing Smoke Reader Beta’s library"
    Select-Person 'Everyone'
    $null = Wait-Element 'Viewing everyone in the household'
    Close-App
    Open-App
    $null = Wait-Element 'Viewing everyone in the household'
    Close-App

    # A deliberately invalid file is created only in another fresh smoke folder.
    $env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
    $corruptDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) "local.bookbuddy.desktop/development/smoke/$env:BOOKBUDDY_TEST_RUN"
    if (Test-Path -LiteralPath $corruptDirectory) { throw 'Refusing to reuse an existing corruption-test folder.' }
    New-Item -ItemType Directory -Path $corruptDirectory | Out-Null
    $corruptFile = Join-Path $corruptDirectory 'bookbuddy.sqlite3'
    [System.IO.File]::WriteAllText($corruptFile, 'invalid synthetic database: preserve this file')
    $before = (Get-FileHash -LiteralPath $corruptFile).Hash
    Open-App
    $null = Wait-Element 'Your library could not be opened'
    Close-App
    if ((Get-FileHash -LiteralPath $corruptFile).Hash -ne $before) { throw 'Invalid database bytes changed.' }
    Write-Output 'PASS: offline create, switch, rename, archive, Everyone, restart persistence and safe startup failure through the Windows UI and typed IPC.'
} finally {
    Close-App
}
