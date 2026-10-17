# BookBuddy

An offline Windows household book tracker. Product requirements are in
[DESIGN.md](DESIGN.md); progress and verification gates are in [PLAN.md](PLAN.md).

## Development on Windows

Install Node.js 24 LTS, Rust stable for `x86_64-pc-windows-msvc`, Visual Studio C++
Build Tools with a Windows SDK, and the Microsoft Edge WebView2 runtime. Dependency
installation requires registry access once; the installed application works offline.

```powershell
npm ci
npm run desktop:dev
```

Build bundled frontend assets and an NSIS installer:

```powershell
npm run desktop:build
```

The installer requires an existing WebView2 runtime and does not download one.
For a fast executable-only build use `npm run tauri -- build --no-bundle`.
`npm run dev` serves the frontend alone; desktop persistence needs Tauri.

## Validation

```powershell
npm run check
```

This runs Prettier, ESLint, strict TypeScript, the frontend production build,
Rust formatting, Clippy with warnings denied, and Rust tests including temporary
SQLite migrations/reopen and invalid-database preservation. Commit both lockfiles.
Use `npm run format` and `cargo fmt --manifest-path src-tauri/Cargo.toml` to format.
After dependencies are cached, append `--offline` to Cargo commands as needed.

Desktop smoke checks must also launch the built executable on Windows, inspect
the shell and restart it. Later phases add household acceptance scenarios.
After a debug Tauri build, run `powershell -File scripts/smoke-shell.ps1`. It
checks the welcome heading through Windows UI Automation with external WebView
requests directed to an unreachable proxy. Run without a Vite server. Plain
`cargo run` builds the development-server variant; use the Tauri build command
before testing bundled assets.

## Local data and privacy

Release data lives in `%LOCALAPPDATA%\local.bookbuddy.desktop\bookbuddy.sqlite3`;
debug data lives in the `development` subdirectory. Both use WAL and foreign keys.
No environment-variable database URL can redirect the application into a household
database. Tests use disposable temporary directories only. Startup failures preserve
existing files. Do not delete data to resolve migration errors.

Never commit real workbooks, databases, backups or exports. `example/` is private
ignored input. Synthetic fixtures belong in `tests/` and must be explicitly allowed
in `.gitignore` if their extension is otherwise excluded. No telemetry or cloud
services are present. People in a household are labels, not private accounts.
