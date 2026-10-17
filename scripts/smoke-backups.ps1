$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$folder = New-Item -ItemType Directory -Force -Path "$PSScriptRoot/../.local/backup-smoke/$env:BOOKBUDDY_TEST_RUN"
$backup = Join-Path $folder.FullName 'synthetic.bookbuddy'
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"
. "$PSScriptRoot/native-file-dialog.ps1"
try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Name-Entry 'Smoke Reader Alpha'
    Click 'Create person'
    Click 'Add book'
    Set-Text 'Title' 'Synthetic backup book'
    Click 'Save book'
    Set-Text "Smoke Reader Alpha's notes" 'A synthetic note retained through restore'
    Click "Save Smoke Reader Alpha's reading information"
    Click 'Log Synthetic backup book read today for Smoke Reader Alpha'
    Click 'Settings'
    Click 'Export backup'
    Select-NativeFile 'Export BookBuddy backup' $backup 'Save'
    $null = Wait-Element 'Backup exported. Save a copy on another device or drive for recovery.'
    if (-not (Test-Path -LiteralPath $backup)) { throw 'The selected export was not written.' }
    Click 'Library'
    Click 'Add book'
    Set-Text 'Title' 'Later synthetic book'
    Click 'Save book'
    Click 'Settings'
    Click 'Choose backup to restore'
    Select-NativeFile 'Choose BookBuddy backup' $backup
    $null = Wait-Element 'Review replacement restore'
    Toggle-Checkbox 'Replace my entire current library with this backup'
    Click 'Confirm replacement restore'
    $null = Wait-Element 'Restore complete'
    Click 'Close BookBuddy'
    $script:application.WaitForExit(5000) | Out-Null
    $script:application = $null
    Open-App
    $null = Wait-Element '1 book'
    Click 'Synthetic backup book'
    Assert-Text "Smoke Reader Alpha's notes" 'A synthetic note retained through restore'
    $null = Wait-Element ("Household: 1 logged readings " + [char]0xB7 + " Last reading: " + [DateTime]::Now.ToString('yyyy-MM-dd'))
    Click 'Settings'
    $spinner = Wait-Element 'Automatic backups to keep' ([System.Windows.Automation.ControlType]::Spinner)
    $spinner.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('2')
    Click 'Save backup settings'
    $null = Wait-Element 'Automatic backup settings saved.'
    if ($spinner.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value -ne '2') { throw 'Backup retention was not saved.' }
    Write-Output 'PASS: native backup export, replacement preview/confirmation, pre-restore recovery backup, offline reopen, notes/readings and automatic settings.'
} finally { Close-App }
