$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$fixture = (Resolve-Path "$PSScriptRoot/../.local/fixtures/import-smoke.xlsx").Path
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"

function Choose-SyntheticWorkbook {
    Click 'Choose Excel workbook'
    $nameCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'Choose Excel library'
    $processCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $script:application.Id
    $condition = New-Object System.Windows.Automation.AndCondition $processCondition, $nameCondition
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $dialog = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
        if ($dialog) { break }
        Start-Sleep -Milliseconds 200
    } until ([DateTime]::UtcNow -gt $deadline)
    if (-not $dialog) { throw 'The native workbook picker was not found.' }
    $fileNameCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'File name:'
    $editType = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ControlTypeProperty), ([System.Windows.Automation.ControlType]::Edit)
    $fileNameCondition = New-Object System.Windows.Automation.AndCondition $fileNameCondition, $editType
    $edit = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $fileNameCondition)
    if (-not $edit) { throw 'The native file-name input was not found.' }
    $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($fixture)
    $openCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'Open'
    $open = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $openCondition)
    if (-not $open) { throw 'The native Open control was not found.' }
    $open.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}
try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    Choose-SyntheticWorkbook
    $null = Wait-Element '2 catalogue rows · 2 historical borrowing rows · 1 title review rows'
    Toggle-Checkbox 'I reviewed the source warnings, date interpretation and borrow-count differences'
    Click 'Refresh import preview'
    Click 'Review final import'
    Click 'Confirm import'
    $null = Wait-Element 'Library'
    $null = Wait-Element '2 books'
    Click 'Settings'
    $null = Wait-Element 'Import History'
    $null = Wait-Element 'Title Verification'
    $null = Wait-Element 'Synthetic board book · open'
    Choose-SyntheticWorkbook
    Click 'Review final import'
    Click 'Confirm import'
    $null = Wait-Element 'This unchanged workbook was already imported. No records were added.'
    Click 'Library'
    $null = Wait-Element '2 books'
    Close-App
    Open-App
    $null = Wait-Element '2 books'
    Write-Output 'PASS: native workbook import, pre-import recovery snapshot, summary/history, title queue, exact reimport and offline restart.'
} finally { Close-App }
