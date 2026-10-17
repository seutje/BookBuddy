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
powershell -File scripts/release-build.ps1
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
It also runs frontend update-flow tests. `node --test scripts/*.test.mjs` checks
release selection and signed manifest generation. See `docs/windows-release.md`
for the dedicated signing key, CI secret and updater deployment.
After dependencies are cached, append `--offline` to Cargo commands as needed.

Desktop smoke checks must also launch the built executable on Windows, inspect
the shell and restart it. Later phases add household acceptance scenarios.
After a debug Tauri build, run `powershell -File scripts/smoke-shell.ps1`. It
checks the welcome heading through Windows UI Automation with external WebView
requests directed to an unreachable proxy. Run without a Vite server. Plain
`cargo run` builds the development-server variant; use the Tauri build command
before testing bundled assets.
`powershell -File scripts/smoke-people.ps1` exercises first-run creation, switching,
rename, archival, Everyone, restarts, and the safe startup error screen through
Windows UI Automation and real IPC, using fresh debug smoke databases.
`powershell -File scripts/smoke-books.ps1` checks title-only add, shared metadata,
person-specific notes, Mark read, searching, assignment and restart isolation.
`powershell -File scripts/smoke-fields.ps1` checks new attributes on old and new
books, Unknown/No/Yes, display-only defaults, filtering, hide/reactivate and rename.
`powershell -File scripts/smoke-readings.ps1` checks shared/repeat readings,
derived counts, edit/delete/undo, recent shortcuts, restart and archived history.
To exercise the native XLSX picker with public synthetic data, first run
`cargo test --manifest-path src-tauri/Cargo.toml --all-features --test xlsx_preview emit_desktop_fixture -- --ignored`,
then `powershell -File scripts/smoke-import-preview.ps1` after a bundled debug build.
The private-workbook aggregate test is opt-in and never prints source rows:
`cargo test --manifest-path src-tauri/Cargo.toml --all-features --test xlsx_preview supplied_private_workbook -- --ignored`.

Library text matching ignores case and repeated whitespace, retains accents and
punctuation, and treats search text literally. Everyone searches notes across the
household; a person's view searches that person's notes and shared metadata.

## Local data and privacy

Release data lives in `%LOCALAPPDATA%\local.bookbuddy.desktop\bookbuddy.sqlite3`;
debug data lives in the `development` subdirectory. Both use WAL and foreign keys.
Debug-only desktop smoke runs set `BOOKBUDDY_TEST_RUN` to a UUID, storing synthetic
data below `development/smoke/<UUID>`. Release builds ignore that variable. Smoke
data is retained for inspection and is never restored into a household database.
No environment-variable database URL can redirect the application into a household
database. Tests use disposable temporary directories only. Startup failures preserve
existing files. Do not delete data to resolve migration errors.

Never commit real workbooks, databases, backups or exports. `example/` is private
ignored input. Synthetic fixtures belong in `tests/` and must be explicitly allowed
in `.gitignore` if their extension is otherwise excluded. No telemetry or cloud
services are present. People in a household are labels, not private accounts.

`scripts/smoke-import-commit.ps1` verifies transactional XLSX import, summary,
provenance history, title queue, exact reimport and offline restart with the same
public synthetic fixture. The private commit/repeat aggregate test is opt-in:
`cargo test --manifest-path src-tauri/Cargo.toml --all-features --test xlsx_commit private_sample_commit -- --ignored`.

See [backup and recovery instructions](docs/backup-restore.md).
`scripts/smoke-backups.ps1` verifies native export, explicit replacement restore,
recovery backup, offline restart and backup retention settings in an isolated library.

## Using and distributing the app

Start with [the user guide](docs/getting-started.md), [XLSX import](docs/library-import.md),
[backup/recovery](docs/backup-restore.md) and [local privacy](docs/privacy.md).
See [Windows packaging and acceptance](docs/windows-release.md) and the
[verification record](docs/verification.md). The current release is 0.1.1.
