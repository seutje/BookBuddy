# ADR 0007: Loss-aware workbook preview and historical provenance

Status: Accepted · 2026-10-17

Read only local XLSX files with Calamine. Bound compressed/expanded size, worksheet
coordinates and cell counts before constructing import records. Never execute
formulas, macros or source text. Preserve each original cell's type, value and
cached formula text; unexpected nonempty columns remain source provenance.

Recognize the five source sheets by normalized name and map columns by normalized
headers. Book Catalogue and its Title mapping are essential. Missing Library
History or Check Titles requires an explicit incomplete-source acknowledgement;
missing Dashboard/How to Use is a reference warning. Ambiguous sheets or headers
block commit. Reference sheets do not create catalog entries.

The native picker and inspection are one narrow command. Subsequent preview and
commit requests use a process-local selection token; the frontend cannot supply
an arbitrary read path. Preview uses read-only database operations and never
creates people, definitions, books or source rows. Commit must recheck the chosen
file hash and the reviewed relevant library state.

Keep borrowing history separate from reading events. Retain the original date
type, precision and PDF label; ambiguous slash dates stay unknown until an explicit
date-order choice. Partial year/month dates stay partial. An unmatched historical
row can remain an unlinked import record rather than inventing a shared book or
guessing an edition. The owner can explicitly link it during or after review.

Preserve independent Carly/Mom ratings, and create Mom memberships only for actual
ratings. Household ownership, source acquisition choices and personal lists remain
separate facts. Existing populated manual values take precedence over imports;
blank source cells never erase values. A confirmed-title correction requires an
explicit reviewed choice, with the working/PDF title retained as evidence.

For repeat imports, keep a stable source origin across explicitly reviewed workbook
revisions. Fingerprint normalized source values by sheet plus duplicate occurrence;
retain physical row numbers separately. Identical-looking loan rows remain distinct.
An exact source-file hash is idempotent. Never fabricate transactions to reconcile
source baseline borrow counts with incomplete reconstructed history.

Reference: DESIGN §§15, 18. Safe commit and persistence are Phase 6 work.

Phase 6 implementation: the source origin, original catalog identities and durable
field/option mappings survive revised files and manual title/choice renames. A
review task uses stable working-title/author/PDF evidence plus occurrence; editing
its confirmed-title slot does not manufacture another task. Saved loan links and
resolved task states are preserved on reimport. Post-import changes use explicit
review with stale-state checks. All source sheets, original cell types, warnings,
review choices and row links are retained. Fill-only merging treats false, zero
and empty saved strings as populated. Pre-import snapshots use SQLite VACUUM INTO
for a consistent view including committed WAL data. The file is hashed before
validation and again immediately before the final transaction commit.
