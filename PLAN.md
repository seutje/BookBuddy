# BookBuddy — Implementation Plan

**Status:** Phases 0–1 verified; Phase 2 next
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

- [ ] Implement book create/edit with title required, optional bibliographic fields, durable ID and safe duplicate hints.
- [ ] Implement person-book association with `UNIQUE(person_id,book_id)` and independent reading status, categorical rating, list status and notes.
- [ ] Implement shared ownership separately from personal library/wishlist/borrowed state; ensure unknown is distinct from false.
- [ ] Build Library search, filter, sort, row view, quick-add and fast `Mark read` action; add clear person-context labels.
- [ ] Build shared Book Detail and per-person panels without accidental cross-person overwrites.
- [ ] Support same book in multiple people's libraries without duplicating the underlying book record.
- [ ] Test one shared book with conflicting ratings and reading statuses; ambiguous titles and absent ISBN; same-person uniqueness.
- [ ] **Exit gate:** title-only add, browse, update, switch member and reopen all work with correct scope.

## Phase 3 — Extensible attributes (required foundation)

- [ ] Migrate `custom_field_definition` plus book-scoped and person-book-scoped value storage.
- [ ] Implement supported types: boolean, select with durable option IDs, number, date, text/multiline, rating.
- [ ] Validate every custom value and its entity scope on the Rust side; reject invalid field or option IDs.
- [ ] Add Settings field-definition create/edit/reorder/hide UI with readable shared/per-person scope selection.
- [ ] Add field rendering and editing to book and per-person detail views; custom values remain optional.
- [ ] Add search/filter for custom values including missing/Unknown distinctly from No, zero and empty string.
- [ ] Support safe label/choice rename; provide impact preview for type changes, removal and destructive operations.
- [ ] Test adding `Recommend?` **after** hundreds of person-book rows already exist; no table migration/bulk fill needed.
- [ ] Test booleans (Unknown/No/Yes), cross-scope misuse, deleted or renamed options, field hide/reactivate and restart.
- [ ] **Exit gate:** any existing and future per-person book can use a newly defined field and be filtered by it.

## Phase 4 — Reading events and household activity

- [ ] Migrate reading events plus participant join table with unique per-event participants.
- [ ] Implement log read today, backdated entry, optional note, edit/delete and event queries.
- [ ] Support one event with multiple household members; enforce participant IDs and transactional updates.
- [ ] Derive last read and read count from reading events; do not infer event dates from a manual read-status change.
- [ ] Build Activity view and recent-title shortcuts for quick repeat toddler sessions.
- [ ] Decide/document prompt to set participant status to Read while preserving all other person-specific values.
- [ ] Test repeat readings, multiple participants, undo/edit, timezone/local-date behavior and persistence.
- [ ] **Exit gate:** same book can be logged repeatedly and each participant's history is accurate.

## Phase 5 — XLSX importer: source ingestion and preview

- [ ] Add offline Rust `.xlsx` reader (e.g. calamine), input constraints and structured parsing warnings.
- [ ] Recognize source tabs: `Book Catalogue`, `Library History`, `Check Titles`, `Dashboard`, `How to Use`; reject ambiguous/missing essential sheets clearly.
- [ ] Detect columns by normalized header name, not hardcoded letter; ignore blank rows; warn on new nonempty columns.
- [ ] Implement all `Book Catalogue` column mappings in DESIGN §15.2, including preservation of original labels and unknowns.
- [ ] Map source people `Carly` and `Mom` to new or existing local members; ratings remain per person and missing Mom values do not fabricate assignments.
- [ ] Parse `Library History` to distinct historical borrowing records, preserving approximate date/date type/source PDF rather than fabricating reading sessions.
- [ ] Parse `Check Titles` into a verifiable task queue, retaining unresolved and ambiguous source rows.
- [ ] Inspect row counts, date-cell types, title/author ambiguity, source counts and borrow-history discrepancies.
- [ ] Implement native Choose File → Inspect → Review preview with **zero writes** to the library during these steps.
- [ ] Test small synthetic workbooks for malformed dates, missing tabs, reordered headers, blanks, unknown booleans and duplicate titles.
- [ ] **Exit gate:** user can see exactly what will change, source warnings and people mappings before committing.

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
