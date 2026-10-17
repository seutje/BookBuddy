$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"
$directory = Get-SmokeDataDirectory
if (Test-Path -LiteralPath $directory) { throw 'Refusing to reuse an existing paging-test database.' }
$null = New-Item -ItemType Directory -Path $directory
# This source is a synthetic consistent VACUUM snapshot, never a live database.
Copy-Item -LiteralPath "$PSScriptRoot/../.local/fixtures/paging.sqlite3" -Destination (Join-Path $directory 'bookbuddy.sqlite3')
try {
    Open-App
    $null = Wait-Element '1001 books'
    $null = Wait-Element 'Synthetic paging 0001' ([System.Windows.Automation.ControlType]::Button)
    if (Find-Element 'Synthetic paging 0101' ([System.Windows.Automation.ControlType]::Button)) { throw 'Library rendered records beyond its first page.' }
    Click 'Next books'
    $null = Wait-Element 'Synthetic paging 0101' ([System.Windows.Automation.ControlType]::Button)
    Set-Text 'Search books' 'Synthetic paging 1001'
    $null = Wait-Element '1 book'
    $null = Wait-Element 'Synthetic paging 1001' ([System.Windows.Automation.ControlType]::Button)
    Click 'Activity'
    $null = Wait-Element '1001 reading events'
    $null = Wait-Element 'Edit reading 100' ([System.Windows.Automation.ControlType]::Button)
    if (Find-Element 'Edit reading 101' ([System.Windows.Automation.ControlType]::Button)) { throw 'Activity rendered records beyond its first page.' }
    Click 'Next readings'
    $null = Wait-Element 'Edit reading 101' ([System.Windows.Automation.ControlType]::Button)
    Write-Output 'PASS: 1001-book/event library has bounded 100-row pages, next-page access, global search and page reset after filtering, offline through Windows UI and IPC.'
} finally { Close-App }
