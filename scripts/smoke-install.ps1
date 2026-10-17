param([Parameter(Mandatory=$true)][ValidateSet('Baseline','Upgrade')][string]$Stage, [Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$workspace = (Resolve-Path "$PSScriptRoot/..").Path
$installDirectory = [IO.Path]::GetFullPath((Join-Path $workspace '.local/installed/BookBuddy'))
if (-not $installDirectory.StartsWith(($workspace + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Installer test destination escaped the workspace.' }
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BookBuddy'
if (Test-Path $registry) {
    $existing = (Get-ItemProperty $registry).InstallLocation.Trim('"')
    if ($existing -ne $installDirectory) { throw 'An unrelated BookBuddy installation exists. The test will not overwrite it.' }
}
if (Test-Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BookBuddy') { throw 'A machine-wide BookBuddy installation exists; preserve it.' }
if (Get-Process -Name bookbuddy -ErrorAction SilentlyContinue) { throw 'Close BookBuddy before testing the installer.' }
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$installed = Start-Process -FilePath $installerPath -ArgumentList @('/S', "/D=$installDirectory") -WindowStyle Hidden -PassThru -Wait
if ($installed.ExitCode -ne 0) { throw 'The silent local installer did not finish successfully.' }
$executable = Join-Path $installDirectory 'bookbuddy.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw 'The installer did not create its executable in the intended folder.' }
$env:BOOKBUDDY_SMOKE_EXECUTABLE = $executable
$env:BOOKBUDDY_SMOKE_INSTALLED = '1'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=localhost;*.localhost;127.0.0.1'
$statePath = Join-Path $workspace '.local/installer-smoke.json'
if ($Stage -eq 'Baseline') {
    $env:BOOKBUDDY_TEST_RUN = [Guid]::NewGuid().ToString()
    @{runId=$env:BOOKBUDDY_TEST_RUN; directory=$installDirectory} | ConvertTo-Json | Set-Content -Encoding UTF8 -LiteralPath $statePath
} else {
    $state = Get-Content -Raw -Encoding UTF8 -LiteralPath $statePath | ConvertFrom-Json
    if ($state.directory -ne $installDirectory) { throw 'The upgrade test folder changed.' }
    $env:BOOKBUDDY_TEST_RUN = ([Guid]::Parse($state.runId)).ToString()
}
$script:application = $null
. "$PSScriptRoot/ui-automation.ps1"
try {
    Open-App
    if ($Stage -eq 'Baseline') {
        $null = Wait-Element 'Welcome to BookBuddy'
        Name-Entry 'Synthetic installer reader'
        Click 'Create person'
        Click 'Add book'
        Set-Text 'Title' 'Survives install and upgrade'
        Click 'Save book'
        Set-Text "Synthetic installer reader's notes" 'Preserved by release installer'
        Click "Save Synthetic installer reader's reading information"
        Click 'Log Survives install and upgrade read today for Synthetic installer reader'
    } else {
        $null = Wait-Element '1 book'
        Click 'Survives install and upgrade'
        Assert-Text "Synthetic installer reader's notes" 'Preserved by release installer'
    }
    Close-App
    Write-Output "PASS: $Stage installer ran offline; installed release opened its isolated persistent dataset and preserved personal book data."
} finally { Close-App }
