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
