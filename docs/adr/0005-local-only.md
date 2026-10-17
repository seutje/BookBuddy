# ADR 0005: Local-only storage and privacy

Status: Accepted · 2026-10-17

People are local organizational labels, with no authentication. Rust owns SQLite
and filesystem operations. React receives narrow typed commands, never arbitrary
SQL or unrestricted file access. No analytics, network services, automatic updates
or metadata downloads are included. All release assets are bundled locally.

Resolve the database through Tauri's local app-data directory under identifier
`local.bookbuddy.desktop`. Debug builds use a `development` subdirectory so normal
development cannot change the release household. Tests use temporary directories.
Enable foreign keys on every connection and use WAL and SQLx migrations. Startup
errors never trigger a reset or deletion. Backups must use consistent snapshots.

Installers skip automatic WebView2 downloading; document the local runtime
prerequisite. This preserves an offline installation path.

Reference: DESIGN §§6, 10, 17.
