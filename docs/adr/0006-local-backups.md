# ADR 0006: Versioned local backups and explicit safe restore

Status: Accepted · 2026-10-17

Use a version 1 `.bookbuddy` ZIP containing `manifest.json` and a consistent SQLite
snapshot, created with `VACUUM INTO`. The manifest records format/schema/app
versions, UTC creation time, database length and SHA-256. All household records,
custom definitions/options/values, reading participants, import evidence, original
source sheets and verification tasks live in that snapshot. Covers are not offered
by this MVP. The manifest's cover list must be empty; non-null cover references
are rejected explicitly rather than silently omitted.

The Rust native picker owns paths. IPC uses expiring process-local selection tokens.
Bound compressed/expanded input, require exactly the two known regular entries,
and never extract archive paths. Validate checksums, application ID, integrity,
foreign keys, migration versions/checksums and the exact schema reconstructed from
the accepted immutable migrations. Upgrade older compatible backups only in a
disposable staging copy. A backup cannot introduce extra triggers or schema objects.

Restore is replacement only. Display current/backup counts and require explicit
replacement confirmation. Recheck both archive hash and the complete current
database state; save a portable pre-restore backup. An application-wide operation
gate waits for other IPC work and excludes new work while SQLite's online backup
API copies the staged snapshot into the existing database. SQLite's destination
write transaction provides atomic commit/rollback and crash recovery. Keep both
SQLx handles locked throughout the minimal FFI scope and always finish the backup.
Pin SQLx/libsqlite3-sys together because raw handle types are semver-exempt.

After success close the pool and require an application restart, discarding old
prepared statements and UI state. Do not replace live WAL files by filesystem copy.
A Windows sharing-denied directory lock prevents a second app instance using the
same dataset and releases automatically on process termination.

Automatic backups run on startup at most every 24 hours, enabled by default with
five retained archives (configurable 1–20 or off). Rotate only recognized automatic
UUID filenames inside the app-owned backup folder. Manual exports and pre-import/
pre-restore recovery files are never pruned. Report automatic failure without
blocking normal library use. Exports use a synced temporary file and no-clobber
installation; existing files survive all failures. Backups are unencrypted local
data, with no remote destination or network dependency.
