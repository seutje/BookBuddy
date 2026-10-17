param([ValidateSet('All','Build','Test')][string]$Stage = 'All')
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
$testRoot = [IO.Path]::GetFullPath((Join-Path $workspace '.local/signed-upgrade'))
if (-not $testRoot.StartsWith(($workspace + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Test destination escaped the workspace.' }
$installRoot = Join-Path $testRoot 'installed'
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BookBuddyUpdaterAcceptance'
if ($Stage -ne 'Build' -and (Get-Process -Name bookbuddy -ErrorAction SilentlyContinue)) { throw 'Close BookBuddy before the isolated installer test.' }
if (Test-Path -LiteralPath $registry) { throw 'An acceptance installation already exists; preserve and review it before retrying.' }
if (Test-Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BookBuddyUpdaterAcceptance') { throw 'An unrelated machine-wide acceptance installation exists.' }
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null
Set-Location $workspace
. "$PSScriptRoot/windows-toolchain.ps1"
if ($Stage -ne 'Test') {
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY) { $env:TAURI_SIGNING_PRIVATE_KEY = Join-Path $workspace '.local/release-secrets/bookbuddy.key' }
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
    foreach ($version in @('0.1.0','0.1.1')) {
        $config = Join-Path $testRoot "acceptance-$version.json"
        $override = @{ productName='BookBuddyUpdaterAcceptance'; identifier='local.bookbuddy.updater-acceptance'; version=$version } | ConvertTo-Json
        [IO.File]::WriteAllText($config, $override, (New-Object Text.UTF8Encoding($false)))
        & npm.cmd run tauri -- build --ci --config $config --bundles nsis -- --locked
        if ($LASTEXITCODE -ne 0) { throw 'Acceptance installer build failed.' }
        $name = "BookBuddyUpdaterAcceptance_${version}_x64-setup.exe"
        Copy-Item -LiteralPath (Join-Path $workspace "src-tauri/target/release/bundle/nsis/$name") -Destination (Join-Path $testRoot $name)
        Copy-Item -LiteralPath (Join-Path $workspace "src-tauri/target/release/bundle/nsis/$name.sig") -Destination (Join-Path $testRoot "$name.sig")
    }
    & cargo build --manifest-path src-tauri/Cargo.toml --example release_acceptance --locked
    if ($LASTEXITCODE -ne 0) { throw 'Acceptance helper build failed.' }
}
if ($Stage -eq 'Build') { return }
$env:BOOKBUDDY_UPGRADE_RUN = [Guid]::NewGuid().ToString()
@{runId=$env:BOOKBUDDY_UPGRADE_RUN} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $testRoot 'run.json')
$data = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) "local.bookbuddy.updater-acceptance/acceptance/$env:BOOKBUDDY_UPGRADE_RUN"
if (Test-Path -LiteralPath $data) { throw 'Isolated test data already exists.' }
function Invoke-Helper([string]$mode) {
    $log = Join-Path $testRoot "$mode.log"
    $helper = Join-Path $workspace 'src-tauri/target/debug/examples/release_acceptance.exe'
    $process = Start-Process -FilePath $helper -ArgumentList $mode -WindowStyle Hidden -PassThru -Wait -RedirectStandardOutput $log -RedirectStandardError (Join-Path $testRoot "$mode-error.log")
    Get-Content -LiteralPath $log
    if ($process.ExitCode -ne 0) { throw "Isolated helper failed: $mode (see ignored local logs)." }
}
function Start-Fixture {
    $process = Start-Process -FilePath (Join-Path $installRoot 'bookbuddy.exe') -ArgumentList "--acceptance-run=$env:BOOKBUDDY_UPGRADE_RUN" -WindowStyle Hidden -PassThru
    Start-Sleep -Seconds 3
    if ($process.HasExited) { throw 'Acceptance app did not remain running.' }
    $null = $process.CloseMainWindow()
    if (-not $process.WaitForExit(10000)) { Stop-Process -Id $process.Id; $process.WaitForExit() }
}
$installer = Join-Path $testRoot 'BookBuddyUpdaterAcceptance_0.1.0_x64-setup.exe'
$process = Start-Process -FilePath $installer -ArgumentList "/S /D=$installRoot" -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw 'Initial acceptance installation failed.' }
Start-Fixture
Invoke-Helper 'seed'
Start-Fixture
Invoke-Helper 'verify'
Invoke-Helper 'install'
$deadline = (Get-Date).AddSeconds(60)
do {
    Start-Sleep -Milliseconds 500
    $version = (Get-ItemProperty -LiteralPath $registry -ErrorAction SilentlyContinue).DisplayVersion
} until ($version -eq '0.1.1' -or (Get-Date) -gt $deadline)
if ($version -ne '0.1.1') { throw 'Signed SDK handoff did not update the isolated installation.' }
if ((Get-Item -LiteralPath (Join-Path $installRoot 'bookbuddy.exe')).VersionInfo.ProductVersion -ne '0.1.1') { throw 'Installed executable was not upgraded.' }
if ((Get-ItemProperty -LiteralPath $registry).InstallLocation.Trim('"') -ne $installRoot) { throw 'Upgrade changed the isolated installation directory.' }
Start-Fixture
Invoke-Helper 'verify'
$process = Start-Process -FilePath (Join-Path $installRoot 'uninstall.exe') -ArgumentList "/S _?=$installRoot" -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw 'Acceptance uninstall failed.' }
Invoke-Helper 'verify'
'PASS: signed SDK download/install, old-to-new upgrade, restart and uninstall preserved the isolated dataset.' | Tee-Object -FilePath (Join-Path $testRoot 'acceptance.txt')
# Retain synthetic data and evidence; never remove a household directory.
