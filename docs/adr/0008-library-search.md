# ADR 0008: Household search and filter semantics

Status: Accepted · 2026-10-17

SQLite remains the source of truth. Library reads a consistent transaction snapshot
and uses bound SQL predicates on built-in membership/ownership columns. All personal
predicates must match the same membership, including in Everyone view. Unknown
means SQL NULL; an unassigned book has no personal values to filter.

Text matching uses Rust Unicode lowercase with whitespace collapsed, retaining
diacritics and punctuation. It searches shared metadata and notes plus the selected
person's notes (all memberships in Everyone view). It never interprets wildcard,
SQL or markup syntax. This avoids SQLite's ASCII-only default lowercase behavior.
Text matching and ordering initially run over the filtered household projection;
profile several thousand records in Phase 8 before introducing a search index.

Duplicate hints use an exact normalized title plus author when supplied, or a
matching ISBN-13. Hints never merge records. Title-only duplicates are permitted.
Categorical rating order is Loved, Liked, Meh, Nope, Unknown last; Everyone uses
the best known category. No numerical rating is stored or implied.

Reference: DESIGN §§2, 8–9.
