# BookBuddy — Implementation Plan

**Status:** Phases 0–5 verified; Phase 6 next
**Target:** Offline-only Windows desktop (Rust + Tauri 2 + React + TypeScript + SQLite)  
**Source of truth:** [DESIGN.md](DESIGN.md)  
**Agent rules:** [AGENTS.md](AGENTS.md)

## How to maintain this plan

- Check off an item (`- [x]`) **only after** the code, relevant tests, and any needed documentation are complete and verified. Leave unchecked if blocked or unverified.
- Work in phase order by default. Complete each phase's exit gate before moving on, or document a reason in `docs/adr/`.
- Each item is a deliverable, not a promise. Add sub-items or concise notes when implementation uncovers meaningful work; do not silently change the product requirements.
- The sample XLSX workbook is sensitive local input: do not commit it or raw imported rows to a public repository.

## Phase 0 — Project decisions and scaffolding

- [x] Create Tauri 2 / React / TypeScript / Rust project with working Windows dev/build commands and pinned lockfiles.
- [x] Set up ESLint/formatting, Rust fmt/clippy, TypeScript strict mode and a single command to run checks.
- [x] Create feature-oriented frontend folders and `commands`, `domain`, `services`, `persistence`, `migrations` Rust folders.
- [x] Record initial ADRs for shared book vs per-person membership, scoped custom fields, reading event semantics, household ownership, and local-only privacy.
- [x] Establish a safe app-data location, reproducible SQLite dev setup, and .gitignore for private databases, workbook samples, backups and generated files.
- [x] Add CI or scripted local validation for TypeScript, Rust and database tests.
- [x] **Exit gate:** blank shell launches in Windows, offline, and developer checks run cleanly.

Phase 0 verification (2026-10-17): `npm run check` passed (Prettier, ESLint, strict TypeScript, Vite build, Rust fmt, Clippy all targets/features, two SQLite integration tests). Windows `tauri build --debug --no-bundle` and `desktop:dev` succeeded. `scripts/smoke-shell.ps1` found the bundled React heading with no dev server and external WebView requests blocked by an unreachable proxy. Installer packaging remains a Phase 8 gate.

## Phase 1 — Persistence, app shell and people

- [x] Implement versioned SQLx SQLite migrations, foreign keys, transaction helpers and connection lifecycle.
- [x] Implement `person`, `book`, `person_book`, household ownership and settings tables with uniqueness and referential constraints.
- [x] Implement startup migration/error reporting and ensure existing data is never silently reset.
- [x] Add typed Tauri IPC commands, typed TypeScript invoke wrappers and structured error contracts.
- [x] Build compact accessible desktop shell: Library, Activity, People, Settings, global person picker and Everyone view.
- [x] Implement create, rename, archive and list people; make Everyone a filter, not a persisted person.
- [x] Add first-run create-person flow and persist the current person preference.
- [x] Test migrations, person rename stability, archived-person behavior, and offline persistence through restart.
- [x] **Exit gate:** a household can create/switch members without accounts, data survives restart, and no network is needed.

Phase 1 verification (2026-10-17): `npm run check` passed, including nine Rust tests for migrations/upgrades, failed migration preservation, scoped memberships, ownership, transaction rollback, archival, stable rename, preference/restart, smoke isolation and redacted IPC errors. `scripts/smoke-people.ps1` passed on Windows with external WebView traffic blocked: create/switch/rename/archive, Everyone, repeated restarts and byte-preserving invalid-database startup. No household database was used.

## Phase 2 — Catalog and person-specific libraries

- [x] Implement book create/edit with title required, optional bibliographic fields, durable ID and safe duplicate hints.
- [x] Implement person-book association with `UNIQUE(person_id,book_id)` and independent reading status, categorical rating, list status and notes.
- [x] Implement shared ownership separately from personal library/wishlist/borrowed state; ensure unknown is distinct from false.
- [x] Build Library search, filter, sort, row view, quick-add and fast `Mark read` action; add clear person-context labels.
- [x] Build shared Book Detail and per-person panels without accidental cross-person overwrites.
- [x] Support same book in multiple people's libraries without duplicating the underlying book record.
- [x] Test one shared book with conflicting ratings and reading statuses; ambiguous titles and absent ISBN; same-person uniqueness.
- [x] **Exit gate:** title-only add, browse, update, switch member and reopen all work with correct scope.

Phase 2 verification (2026-10-17): `npm run check` passed, including fourteen Rust tests. New coverage includes independent ratings/status/notes, title-only duplicates and ISBN hints without merging, transactional failed assignment, archived write rejection, Unicode case-insensitive scoped search, combined same-membership filters, three-valued ownership and categorical sorting. `scripts/smoke-books.ps1` passed on Windows with external WebView requests blocked: title-only add, personal notes, Mark read preserving notes, shared edits, search, assignment to a second person, context switching and restart. Search semantics are recorded in ADR 0008.

## Phase 3 — Extensible attributes (required foundation)

- [x] Migrate `custom_field_definition` plus book-scoped and person-book-scoped value storage.
- [x] Implement supported types: boolean, select with durable option IDs, number, date, text/multiline, rating.
- [x] Validate every custom value and its entity scope on the Rust side; reject invalid field or option IDs.
- [x] Add Settings field-definition create/edit/reorder/hide UI with readable shared/per-person scope selection.
- [x] Add field rendering and editing to book and per-person detail views; custom values remain optional.
- [x] Add search/filter for custom values including missing/Unknown distinctly from No, zero and empty string.
- [x] Support safe label/choice rename; provide impact preview for type changes, removal and destructive operations.
- [x] Test adding `Recommend?` **after** hundreds of person-book rows already exist; no table migration/bulk fill needed.
- [x] Test booleans (Unknown/No/Yes), cross-scope misuse, deleted or renamed options, field hide/reactivate and restart.
- [x] **Exit gate:** any existing and future per-person book can use a newly defined field and be filtered by it.

Phase 3 verification (2026-10-17): `npm run check` passed, including twenty-two Rust tests. Added eight custom-field tests covering sparse availability after 300 memberships, all supported value kinds, SQL/Rust scope checks, Unknown/No/Yes/zero/empty text, option identity and retirement, hide/reactivate, stale previews, rollback of destructive changes, label collisions, reorder, unit-label preservation and tighter-range review. `scripts/smoke-fields.ps1` passed on Windows offline: old/new books, display-only defaults, boolean edits and filters, rename, hide/reactivate and restart. Removal is implemented as non-destructive hiding/choice retirement; no hard-delete command is exposed. Type changes and tighter validation require explicit impact confirmation; see ADR 0002.

## Phase 4 — Reading events and household activity

- [x] Migrate reading events plus participant join table with unique per-event participants.
- [x] Implement log read today, backdated entry, optional note, edit/delete and event queries.
- [x] Support one event with multiple household members; enforce participant IDs and transactional updates.
- [x] Derive last read and read count from reading events; do not infer event dates from a manual read-status change.
- [x] Build Activity view and recent-title shortcuts for quick repeat toddler sessions.
- [x] Decide/document prompt to set participant status to Read while preserving all other person-specific values.
- [x] Test repeat readings, multiple participants, undo/edit, timezone/local-date behavior and persistence.
- [x] **Exit gate:** same book can be logged repeatedly and each participant's history is accurate.

Phase 4 verification (2026-10-17): `npm run check` passed with twenty-eight Rust tests, including six reading tests for shared/repeat sessions, derived counts, edit/delete, transactional failures, archival, local-date offset boundaries and scoped history filters/sorts. `scripts/smoke-readings.ps1` passed on Windows with external WebView traffic blocked: status without an invented date, repeated quick logs, shared participants, edit/delete/undo, recent-title shortcuts, preservation of unsaved personal notes, restart and history after everyone is archived. ADR 0003 records the explicit set-Read behavior and archive/edit semantics.

## Phase 5 — XLSX importer: source ingestion and preview

- [x] Add offline Rust `.xlsx` reader (e.g. calamine), input constraints and structured parsing warnings.
- [x] Recognize source tabs: `Book Catalogue`, `Library History`, `Check Titles`, `Dashboard`, `How to Use`; reject ambiguous/missing essential sheets clearly.
- [x] Detect columns by normalized header name, not hardcoded letter; ignore blank rows; warn on new nonempty columns.
- [x] Implement all `Book Catalogue` column mappings in DESIGN §15.2, including preservation of original labels and unknowns.
- [x] Map source people `Carly` and `Mom` to new or existing local members; ratings remain per person and missing Mom values do not fabricate assignments.
- [x] Parse `Library History` to distinct historical borrowing records, preserving approximate date/date type/source PDF rather than fabricating reading sessions.
- [x] Parse `Check Titles` into a verifiable task queue, retaining unresolved and ambiguous source rows.
- [x] Inspect row counts, date-cell types, title/author ambiguity, source counts and borrow-history discrepancies.
- [x] Implement native Choose File → Inspect → Review preview with **zero writes** to the library during these steps.
- [x] Test small synthetic workbooks for malformed dates, missing tabs, reordered headers, blanks, unknown booleans and duplicate titles.
- [x] **Exit gate:** user can see exactly what will change, source warnings and people mappings before committing.

Phase 5 verification (2026-10-17): `npm run check` passed with thirty-five regular Rust tests (two opt-in XLSX tests excluded from the default run). Seven preview tests exercise all sheets/mappings, header shifts/reordering, source identity, unknown columns, malformed/missing/ambiguous input, date systems/precision, ratings and booleans; preview also succeeds against a SQLite read-only pool. The supplied private-workbook aggregate test was explicitly run and passed all documented counts without printing source rows. The synthetic-fixture emitter and `scripts/smoke-import-preview.ps1` passed, including native selection, people-mapping refresh and zero library/definition writes. ADR 0007 records source/provenance decisions and the opaque file-selection boundary. Transactional commit remains Phase 6.

## Phase 6 — XLSX importer: safe commit and verification

- [ ] Create import batch/source-row provenance and stable deduplication identity for exact reimport, including repeated identical-looking history rows.
- [ ] Implement reviewed resolution for confirmed matches, ambiguous matches, duplicate candidate books and title-correction proposals.
- [ ] Implement new vs existing value policy: keep later manual edits by default; imported blanks never erase filled values.
- [ ] Recheck selected file hash immediately before commit to detect changes since preview.
- [ ] Take pre-import recovery backup and commit linked book/person/field/history/task changes in a transaction; roll back on error.
- [ ] Implement visible import summary and Import History; expose Needs title verification filter and resolution workflow.
- [ ] Verify sample-data invariants: 312 catalog rows, 253 history rows, 35 title-review rows, 66 owned and 3 currently borrowed as observed; never manufacture reading events or missing ratings.
- [ ] Verify exact same workbook twice gives no extra rows; deliberate repeated loans remain separate distinct source events.
- [ ] Verify corrected-title/revised-workbook import previews changes without silently losing user edits.
- [ ] **Exit gate:** provided workbook imports locally and loss-aware, is safely repeatable and remains traceable to its rows.

## Phase 7 — Backups, restores and data safety

- [ ] Define versioned portable backup format containing all SQLite records and optional local cover assets with a manifest/schema version.
- [ ] Implement consistent snapshot/export (SQLite backup API or equivalent), not unsafe copying of live WAL files.
- [ ] Implement restore validation and preview; MVP restore may replace the dataset but must clearly say so.
- [ ] Create pre-restore backup and transactional/atomic recovery procedure; test interruptions and invalid archives.
- [ ] Add a modest configurable local backup rotation, manual export, visible app-data location and offline recovery instructions.
- [ ] Test export/restore round trip for books, people, custom values, histories, provenance and verification tasks.
- [ ] **Exit gate:** user can recover the complete library from a locally saved backup.

## Phase 8 — Usability, hardening and Windows distribution

- [ ] Perform keyboard-only walkthrough of Library, People, Activity, custom-field editor and import wizard; fix focus/labels/contrast.
- [ ] Add useful loading/empty/error states, actionable validation messages and compact toddler-friendly quick actions.
- [ ] Test security boundaries: narrow Tauri capabilities, no generic SQL IPC, no execution of imported HTML, safe path/file handling.
- [ ] Profile local search/filter with several thousand books/events and optimize only evidenced hotspots.
- [ ] Write first-run help, library-import guide, backup/restore guide and data-privacy explanation.
- [ ] Package and smoke-test an installable Windows release; verify app data survives reinstall/upgrade.
- [ ] Run final offline acceptance suite covering core flows, import/reimport, reading history and backup/restore.
- [ ] **Exit gate / MVP:** every outcome in DESIGN §13 and §18 is demonstrably working on Windows without accounts or cloud.

## Deferred ideas (not MVP checkboxes)

Optional ISBN lookup, remote covers, barcode scanning, generic CSV imports, reading goals and cross-device synchronization require separate decisions. Do not implement opportunistically.
