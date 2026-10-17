# Read-only UI check of the Phase 0 shell, using its isolated debug data folder.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$executable = (Resolve-Path "$PSScriptRoot/../src-tauri/target/debug/bookbuddy.exe").Path
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$application = Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $found = $false
    do {
        Start-Sleep -Milliseconds 500
        $application.Refresh()
        if ($application.HasExited) { throw 'BookBuddy exited during startup.' }
        $processCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $application.Id
        $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $processCondition)
        if ($window) {
            $condition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'Welcome to BookBuddy'
            $heading = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
            if ($heading) { $found = $true }
        }
    } until ($found -or [DateTime]::UtcNow -gt $deadline)
    if (-not $found) {
        if ($window) {
            $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { Write-Output $_.Current.Name }
        }
        throw 'The bundled React welcome heading was not found.'
    }
    Write-Output 'PASS: Windows Tauri window rendered the bundled React shell without a development server.'
} finally {
    if (-not $application.HasExited) {
        $null = $application.CloseMainWindow()
        if (-not $application.WaitForExit(5000)) { $application.Kill() }
    }
    $application.Dispose()
}
