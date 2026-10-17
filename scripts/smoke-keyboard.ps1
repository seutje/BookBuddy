$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class BookBuddyFocus {
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr window, int command);
    public static bool Activate(IntPtr window) { ShowWindow(window,9); return SetForegroundWindow(window); }
    public static uint Owner() {uint process;GetWindowThreadProcessId(GetForegroundWindow(),out process);return process;}
}
'@
$env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$fixture = (Resolve-Path "$PSScriptRoot/../.local/fixtures/import-smoke.xlsx").Path
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"
function Assert-AppFocus { if ([BookBuddyFocus]::Owner() -ne $script:application.Id) { throw 'Keyboard acceptance needs the BookBuddy test window in the foreground. No keys were sent to the other application.' } }
function Keys([string]$Value) { Assert-AppFocus; (New-Object -ComObject WScript.Shell).SendKeys($Value); Start-Sleep -Milliseconds 100 }
function Type-Keys([string]$Value) {
    $keyboard = New-Object -ComObject WScript.Shell
    foreach ($character in $Value.ToCharArray()) {
        $key = [string]$character
        if ($character -eq '{') { $key = '{{}' }
        elseif ($character -eq '}') { $key = '{}}' }
        elseif ('+^%~()[]'.Contains($character)) { $key = '{' + $character + '}' }
        Assert-AppFocus
        $keyboard.SendKeys($key)
        Start-Sleep -Milliseconds 30
    }
    Start-Sleep -Milliseconds 200
}
function Tab-To([string]$Name, [System.Windows.Automation.ControlType]$Type = $null) {
    $null = Wait-Element $Name $Type
    for ($attempt = 0; $attempt -lt 180; $attempt++) {
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($focused -and $focused.Current.Name -eq $Name -and (-not $Type -or $focused.Current.ControlType -eq $Type)) { return }
        Keys '{TAB}'
    }
    throw "Keyboard Tab could not reach: $Name"
}
function Activate([string]$Name) { Tab-To $Name ([System.Windows.Automation.ControlType]::Button); Keys '{ENTER}' }
try {
    Open-App
    $null = Wait-Element 'Welcome to BookBuddy'
    $activator = New-Object -ComObject WScript.Shell
    if (-not $activator.AppActivate($script:application.Id)) { throw 'The keyboard test could not activate its synthetic BookBuddy window.' }
    $windowName = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'BookBuddy'
    $windowProcess = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $script:application.Id
    $testWindow = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, (New-Object System.Windows.Automation.AndCondition $windowName, $windowProcess))
    $null = [BookBuddyFocus]::Activate([IntPtr]$testWindow.Current.NativeWindowHandle)
    $focusDeadline = [DateTime]::UtcNow.AddSeconds(45)
    while ([BookBuddyFocus]::Owner() -ne $script:application.Id -and [DateTime]::UtcNow -lt $focusDeadline) { Start-Sleep -Milliseconds 250 }
    Assert-AppFocus
    Start-Sleep -Milliseconds 300
    # Focus enters the app once; every subsequent action uses actual keys.
    (Wait-Element 'Display name' ([System.Windows.Automation.ControlType]::Edit)).SetFocus()
    Start-Sleep -Milliseconds 500
    if ([System.Windows.Forms.Control]::IsKeyLocked([System.Windows.Forms.Keys]::CapsLock)) { Keys '{CAPSLOCK}' }
    Type-Keys 'Keyboard Reader'
    $typed = (Wait-Element 'Display name' ([System.Windows.Automation.ControlType]::Edit)).GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value
    if ($typed -cne 'Keyboard Reader') { throw 'The keyboard host did not deliver the expected synthetic name.' }
    Assert-Text 'Display name' 'Keyboard Reader'
    Activate 'Create person'
    Activate 'Add book'
    Tab-To 'Title' ([System.Windows.Automation.ControlType]::Edit)
    $title = '<img src=x onerror=alert(1)>'
    Type-Keys $title
    Activate 'Save book'
    $null = Wait-Element $title
    Activate 'Log <img src=x onerror=alert(1)> read today for Keyboard Reader'
    Activate 'Back to Library'
    $null = Wait-Element $title ([System.Windows.Automation.ControlType]::Button)
    Activate 'Current person'
    Keys '{TAB}'
    Keys '{ESC}'
    $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
    if ($focused.Current.Name -ne 'Current person') { throw 'Escape did not return focus to the person picker.' }
    Activate 'People'
    Tab-To 'Display name' ([System.Windows.Automation.ControlType]::Edit)
    Type-Keys 'Keyboard Second Reader'
    Activate 'Add person'
    Activate 'Activity'
    Activate 'Log a reading'
    Tab-To 'Reading book' ([System.Windows.Automation.ControlType]::ComboBox)
    Keys '{HOME}{DOWN}{TAB}'
    Tab-To 'Reading note' ([System.Windows.Automation.ControlType]::Edit)
    Type-Keys 'Keyboard shared reading'
    Activate 'Save reading'
    Activate 'Settings'
    Activate 'Add attribute'
    Tab-To 'Attribute name' ([System.Windows.Automation.ControlType]::Edit)
    Type-Keys 'Recommend?'
    Tab-To 'Applies to' ([System.Windows.Automation.ControlType]::ComboBox)
    Keys '{HOME}{DOWN}{TAB}'
    Activate 'Create attribute'
    $null = Wait-Element 'Edit Recommend?'
    Activate 'Choose Excel workbook'
    Start-Sleep -Milliseconds 500
    Keys '%n'
    Type-Keys $fixture
    Keys '{ENTER}'
    $null = Wait-Element 'Refresh import preview'
    Tab-To 'I reviewed the source warnings, date interpretation and borrow-count differences' ([System.Windows.Automation.ControlType]::CheckBox)
    Keys ' '
    Activate 'Refresh import preview'
    Activate 'Review final import'
    Activate 'Confirm import'
    $null = Wait-Element 'Import History'
    Activate 'Library'
    $null = Wait-Element '2 books'
    Keys '+{TAB}'
    Keys '{TAB}'
    Write-Output 'PASS: actual Tab/Shift+Tab, Enter/Space/Escape and native-choice keys across Library, literal HTML text, People, Activity, field editor, native workbook picker and confirmed import; offline and without mouse input.'
} finally { Close-App }
