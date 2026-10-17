# ADR 0005: Local-only storage and privacy

Status: Accepted · 2026-10-17

People are local organizational labels, with no authentication. Rust owns SQLite
and filesystem operations. React receives narrow typed commands, never arbitrary
SQL or unrestricted file access. No analytics, network services, automatic updates
or metadata downloads are included. All release assets are bundled locally.

Resolve the database through Tauri's local app-data directory under identifier
`local.bookbuddy.desktop`. Debug builds use a `development` subdirectory so normal
development cannot change the release household. Tests use temporary directories.
Desktop smoke runs use a validated UUID leaf below `development/smoke/`, selected
by `BOOKBUDDY_TEST_RUN` in debug builds only. This permits repeat/restart checks
without touching development or household data. Release builds ignore that value;
neither build accepts an arbitrary filesystem path or database URL.
Enable foreign keys on every connection and use WAL and SQLx migrations. Startup
errors never trigger a reset or deletion. Backups must use consistent snapshots.

Installed-release acceptance tests use an explicit `--acceptance-run=<UUID>`
argument. It selects only a UUID leaf under the dedicated `acceptance` subfolder;
invalid/duplicate IDs fail startup. It never accepts a path or changes the normal
household location. Release builds continue to ignore the debug environment
override. This allows real installer/upgrade/offline tests without opening a
household database.

Installers skip automatic WebView2 downloading; document the local runtime
prerequisite. This preserves an offline installation path.

Reference: DESIGN §§6, 10, 17.
