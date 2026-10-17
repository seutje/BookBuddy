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
    $null = Wait-Element 'Create person'
    Set-Text 'Carly local name' 'Preview-only reader'
    Click 'Refresh import preview'
    $null = Wait-Element '2 catalogue rows · 2 historical borrowing rows · 1 title review rows'
    Assert-Text 'Carly local name' 'Preview-only reader'
    Click 'Close preview'
    Name-Entry 'Smoke Reader Alpha'
    Click 'Create person'
    $null = Wait-Element 'No books here yet'
    Click 'Settings'
    $null = Wait-Element 'Add attribute'
    if (Find-Element 'Edit Favourite?' ([System.Windows.Automation.ControlType]::Button)) { throw 'Preview unexpectedly created a custom definition.' }
    Write-Output 'PASS: native local workbook picker, five-sheet inspection, people mapping refresh and zero library/definition writes through offline Windows UI and IPC.'
} finally { Close-App }
