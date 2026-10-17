# ADR 0002: Scoped definitions and sparse typed values

Status: Accepted · 2026-10-17

Custom fields have durable definition and option IDs, fixed book or person-book
scope, and sparse value tables for each scope. Use canonical JSON values validated
by Rust against the definition on every write. No dynamic schema columns or eager
bulk value rows are created when a definition is added.

Missing values remain Unknown, distinct from false, zero and empty text. Defaults
are display-only. Labels can change without changing identity. Hide definitions
instead of deleting data; destructive changes require an impact preview.

Reference: DESIGN §§5–6.

Implementation notes (Phase 3): choice retirement and field hiding preserve saved
values; no hard-delete command is exposed. Scope changes are rejected while values
exist. Type changes and tighter validation ranges clear values only after an
explicit impact review. Unit-label changes and relaxed bounds preserve values.
The preview token binds the normalized draft, definition revision and value count;
every value write increments the revision, so a stale review cannot commit. Defaults
remain display suggestions and do not affect Unknown filters. SQLite scope triggers
back up Rust validation, and custom predicates share the same person-book join as
built-in filters.
