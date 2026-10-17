function Find-Element([string]$Name, [System.Windows.Automation.ControlType]$Type = $null) {
    $processCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $script:application.Id
    $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $processCondition)
    if (-not $window) { return $null }
    $condition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $Name
    if ($Type) {
        $typeCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ControlTypeProperty), $Type
        $condition = New-Object System.Windows.Automation.AndCondition $condition, $typeCondition
    }
    return $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Wait-Element([string]$Name, [System.Windows.Automation.ControlType]$Type = $null) {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $element = Find-Element $Name $Type
        if ($element) { return $element }
        if ($script:application.HasExited) { throw 'App exited during smoke test.' }
        Start-Sleep -Milliseconds 200
    } until ([DateTime]::UtcNow -gt $deadline)
    $processCondition = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), $script:application.Id
    $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $processCondition)
    if ($window) {
        $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { Write-Host $_.Current.ControlType.ProgrammaticName $_.Current.Name }
    }
    throw "Expected accessible control was not found: $Name"
}

function Click([string]$Name) {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 100
        try {
            $element = Wait-Element $Name ([System.Windows.Automation.ControlType]::Button)
            if (-not $element.Current.IsEnabled) { continue }
            $pattern = $null
            if ($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) { $pattern.Invoke(); return }
            if ($element.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$pattern)) { $pattern.Expand(); return }
            if ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) { $pattern.Toggle(); return }
            throw "Control cannot be invoked: $Name"
        } catch [System.Management.Automation.MethodInvocationException] {
            if ($_.Exception.InnerException -isnot [System.Windows.Automation.ElementNotAvailableException] -and $_.Exception.InnerException -isnot [System.Windows.Automation.ElementNotEnabledException]) { throw "Could not activate ${Name}: $($_.Exception.Message)" }
            # A query refresh may replace an accessibility node; reacquire it.
        }
    } until ([DateTime]::UtcNow -gt $deadline)
    throw "Control did not become available and enabled: $Name"
}

function Name-Entry([string]$Value) {
    $edit = Wait-Element 'Display name' ([System.Windows.Automation.ControlType]::Edit)
    $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Value)
}

function Select-Person([string]$Name) {
    Click 'Current person'
    Click "View $Name"
}

function Toggle-Checkbox([string]$Name) {
    $element = Wait-Element $Name ([System.Windows.Automation.ControlType]::CheckBox)
    $element.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
}

function Assert-Pressed([string]$Name) {
    $element = Wait-Element $Name ([System.Windows.Automation.ControlType]::Button)
    if ($element.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -ne [System.Windows.Automation.ToggleState]::On) { throw "Expected selection is not active: $Name" }
}
function Open-App {
    $script:application = Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
}

function Close-App {
    if ($script:application -and -not $script:application.HasExited) {
        $null = $script:application.CloseMainWindow()
        if (-not $script:application.WaitForExit(5000)) { $script:application.Kill() }
    }
}

function Set-Text([string]$Label, [string]$Value) {
    $edit = Wait-Element $Label ([System.Windows.Automation.ControlType]::Edit)
    $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($Value)
}
function Assert-Text([string]$Label, [string]$Value) {
    $edit = Wait-Element $Label ([System.Windows.Automation.ControlType]::Edit)
    if ($edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value -ne $Value) { throw "Wrong value in $Label" }
}
