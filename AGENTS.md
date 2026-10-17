# BookBuddy — Agent Working Guide

This repository is a personal, **offline-only Windows desktop book tracker**. It supports a shared catalog, separate per-person reading information, repeat reading events, dynamic custom fields, and import of a particular five-sheet `.xlsx` family library workbook. It is not a cloud product.

## Source documents and precedence

1. `DESIGN.md` is authoritative for product behavior, source-workbook interpretation, data invariants and technical architecture.
2. `PLAN.md` specifies ordered work and checkboxes. Only mark a task done when implementation **and applicable tests** are verified.
3. `docs/adr/` records explicit architectural decisions. If a design choice conflicts with the design, stop and update the decision and design rather than silently diverging.
4. This `AGENTS.md` specifies how to work, test and report changes. If instructions conflict, favor safety, explicit data preservation, and the product's local-only requirements; raise the conflict.

## Required agent workflow

- Read the relevant sections of `DESIGN.md` and `PLAN.md` before editing. Identify the smallest complete, testable vertical slice.
- Inspect existing code/migrations first; reuse patterns rather than introducing competing abstractions.
- Make limited, coherent changes; keep Rust business rules and persistence separate from React presentation.
- Add tests for every changed invariant and import mapping. Run relevant tests, formatting and static checks; say exactly what was and was not run.
- Update the checkboxes in `PLAN.md` only after verifying acceptance. Record blockers clearly; do not claim partially implemented work is complete.
- Include a short final change summary, files changed, tests, and remaining risks or next unchecked items.
- Never modify or destroy a household database or import without explicit review and recovery strategy.

## Absolute product constraints

- **No accounts, authentication, analytics, hosted storage, telemetry, mandatory network calls or cloud sync.** Multiple people are labels/profiles within one local household database; they are not access-controlled identities.
- **Offline core:** add/search/edit books, change personal attributes, log readings, import XLSX and back up/restore must work without internet.
- **Privacy:** never log book notes, personal names, raw XLSX rows, local file paths or exported data into remote telemetry; never add real household workbooks or database backups to the public repo.
- **SQLite is source of truth**, held in the local Windows app-data directory. Use SQLx migration discipline, foreign keys, unique constraints, transactions and consistent backups.
- **Least privilege:** expose narrow typed Tauri IPC commands, not generic SQL or unrestricted filesystem access. Treat workbook cells and any external strings as untrusted text.

## Domain invariants — do not regress

- A `book` is shared bibliographic identity. `person_book` is a person's own reading status, rating, list status, notes and applicable custom values. Enforce unique `(person_id, book_id)`.
- One title can have multiple participants with independent statuses and ratings. Do not copy one person's status or notes to another.
- Household ownership is different from an individual's Library/Wishlist/Borrowed membership. `No` ownership does not imply wishlist.
- Unknown/unset is distinct from false, zero, empty text or `Nope`. Rating choices for the provided workbook are `Loved`, `Liked`, `Meh`, `Nope`; do not force numerical mapping.
- Custom field definitions (stable ID, scope, type, options and order) become available to all applicable existing/new records **without schema changes or eager bulk value rows**. Validate type and scope in Rust; select options retain stable IDs.
- Reading status is not a reading event. `Mark read` must not invent a date. One event can have multiple participants; repeated readings are valid.
- Adding a book to the catalog does not automatically assign it to every person. Archiving someone must not erase shared books or unrelated reading activity.
- Do not silently merge title duplicates or editions based only on approximate title/author comparison.

## Excel import contract — essential

- Support the specific five-sheet workbook described in DESIGN §15: `Book Catalogue`, `Library History`, `Check Titles`, `Dashboard`, `How to Use`.
- Import mappings come from column **headers**, not sheet positions; preserve original source values and unexpected nonempty columns via provenance or warnings.
- `Book Catalogue` includes Carly's reading status and separate Carly/Mom categorical ratings; Mom ratings never imply she read the book. Blank ratings remain unknown.
- `Library History` is **historical borrowing**, **not** reading sessions. Approximate dates may actually be return-date proxies. Retain the original date type and source PDF; never label them verified checkout/read dates.
- `Check Titles` is a review queue. Do not replace titles with fuzzy guesses; require review for an uncertain merge or correction.
- Source catalog borrow counts are baselines and can differ from reconstructed historical rows. Report mismatches; do not fabricate the missing transactions.
- Inspect/preview is read-only; show detected rows, mapping, people and collisions. Recheck file hash before commit. Back up before a transactional commit; roll back on failure.
- An unchanged workbook imported again must be idempotent. Identical-looking legitimate historical loan rows must still remain separate if they were distinct source rows.
- Do not overwrite manual user changes by default. Imported blank cells must not replace populated values. Any destructive option needs an explicit preview and confirmation.
- Never commit raw private household XLSX files or exported snapshots to version control. Use synthetic fixtures for public tests.

## Architecture and implementation conventions

```text
src/app/               React shell, routing, current person, query client
src/features/          library, books, people, activity, custom-fields, import, settings
src/components/        reusable accessible controls
src/lib/               typed Tauri invoke wrappers and display helpers
src-tauri/src/commands/       narrow IPC adapters
src-tauri/src/domain/         structs, IDs, enums, validation
src-tauri/src/services/       household operations and XLSX mapping
src-tauri/src/persistence/    SQLx repositories and transaction orchestration
src-tauri/migrations/         immutable sequential migrations
src-tauri/capabilities/       explicit least-privilege permissions
tests/                integration/e2e/synthetic fixture coverage
docs/adr/             short accepted architectural decisions
```

- Use TypeScript strict typing; avoid `any` at command boundaries. Validate every Rust-side write regardless of UI validation.
- Keep SQL and migrations in persistence; no ad-hoc query building from field names/labels or raw user text.
- Use durable IDs for people, books, field definitions and options. Separate source import row identity from title similarity.
- Use local calendar dates and describe any partial/approximate source-date precision explicitly. Do not round ambiguous dates into certainty.
- Include accessible keyboard navigation and clear indication of which person's data is being edited. A forest-green compact sidebar and simple, low-friction workflows are preferred; do not add dashboards for their own sake.
- Keep the initial application dependably simple: no remote metadata fetching, automated cover downloads, AI requirements or update services by default.

## Verification expectations

- Typical checks after implementation: formatting/linting, TypeScript typecheck, Rust fmt/clippy/tests, SQLx migration/integration tests, and Windows Tauri build/smoke test where available.
- Core tests: one book/two people; field added after existing books; Unknown vs No; one shared reading event/multiple participants; repeat events; historical borrowing distinct from actual reading; safe archival.
- Import tests: all five sheets, header shifts, missing optional/required sheets, date variants, blank ratings, title review, history duplicates, idempotent repeat import, interrupted transaction and pre-commit file hash mismatch.
- Recovery tests: backup consistent with WAL mode; restore round-trip with custom field definitions, original source rows and history; existing data remains safe on failure.
- If the current OS cannot run Windows desktop E2E, state the limitation and leave those plan checkboxes unchecked until a Windows test is performed.

## Completion reporting template

At the end of a coding session report: **Implemented**, **Tests actually run and results**, **Plan checkboxes updated**, **Known limitations**, **Next task**. Do not mark a feature complete solely because UI scaffolding exists.
