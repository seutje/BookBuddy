$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/windows-toolchain.ps1"
Set-Location "$PSScriptRoot/.."
foreach ($step in @('format:check', 'lint', 'typecheck', 'test', 'build')) {
    & npm.cmd run $step
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
& cargo fmt --manifest-path src-tauri/Cargo.toml --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo test --manifest-path src-tauri/Cargo.toml --locked --all-features
exit $LASTEXITCODE
