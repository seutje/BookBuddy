function Select-NativeFile([string]$Title, [string]$Filename, [string]$Action = 'Open') {
    $nameCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $Title
    $processCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $script:application.Id
    $condition = New-Object System.Windows.Automation.AndCondition $processCondition, $nameCondition
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $dialog = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
        if ($dialog) { break }
        Start-Sleep -Milliseconds 200
    } until ([DateTime]::UtcNow -gt $deadline)
    if (-not $dialog) { throw 'The expected native file picker was not found.' }
    $name = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), 'File name:'
    $type = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ControlTypeProperty), ([System.Windows.Automation.ControlType]::Edit)
    $edit = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.AndCondition $name, $type))
    if (-not $edit) { throw 'The native file-name input was not found.' }
    $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Filename)
    $actionCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $Action
    $button = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $actionCondition)
    if (-not $button) { throw 'The native file confirmation control was not found.' }
    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}
