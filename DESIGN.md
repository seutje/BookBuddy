# BookBuddy — Master Design

**Status:** Project source-of-truth specification · 2026-10-17  
**Target:** Windows desktop, one household, offline-first  
**Stack:** Rust, Tauri 2, React, TypeScript, SQLite  
**Audience:** Owner, future maintainers, coding agents  

## 1. Product vision

BookBuddy is a small personal desktop app for tracking books and reading across household members. A parent might begin by logging books read with a toddler, then add herself or other household members without creating accounts or duplicating the book catalog. It is a tailor-made local application, not a commercial product. Favor ease of entry, speed, reliable local storage, flexible attributes, and recoverable data over integrations and scale.

**Core loop:** find or create a book → add it to one or more people's libraries → update status, rating, collection, or notes → optionally log reading events → find books again using search and filters.

### Primary goals

1. Add a book with title alone; enrich metadata later.
2. Track personal reading separately for each household member, without accounts or authentication.
3. Ship useful default fields and let the owner add fields such as `Recommend?` that become available on all applicable records without migrations.
4. Find and update a book quickly; optimized for repeated reading with a toddler.
5. Maintain a household collection and personal lists with unambiguous meaning.
6. Support local backup and portable export with no dependency on the network.
7. Import the supplied multi-sheet XLSX library workbook through a preview-and-confirm workflow with no silent loss of historical context.

### Non-goals for MVP

- Login, identity provider, remote user management, cloud sync, web app or multi-device collaboration.
- Public reviews, followers, social feeds, monetization, recommendation algorithms.
- Mandatory ISBN, book data API, cover retrieval, ISBN scanner or cloud integrations. XLSX import from the supplied household-library format **is in scope for MVP**.
- Complex reading challenges, streaks, analytics or notifications.
- Automatic inference that a book was read merely because it was added to a library.

## 2. Central data-model decision

A **book** is a shared bibliographic entry (title, author, edition, ISBN, cover, etc.). A **person_book** is a particular household member's relationship to that book (read state, rating, wishlist, notes, personal fields). A **reading_event** is a dated instance of reading it, useful for books read repeatedly. Custom field definitions specify which entity type their values attach to.

Do not create a separate duplicate book row for each household member. Conversely, do not put each person's reading status or rating directly on the shared book row.

An optional **household copy/collection** concept records whether the household physically owns a copy; personal `list_status` records whether a person considers the book part of their library, borrowed, or wishlist. These are separate facts.

### Examples

- One book, two person_book records: toddler = read, rating 5, 18 sessions; mother = reading, rating unknown, recommend yes.
- Mother wants a book on her wishlist, while the household already owns a copy: both facts can coexist.
- Owner adds `Recommend?` (boolean, person-book scope): field appears for all existing and new person-book entries, with initial value **Unknown**, not No.
- Owner adds `Illustrator` (text, book scope): one value is shared by all household members.

## 3. UX and navigation

**Primary navigation:** Library · Activity · People · Settings. Global current-person switcher near the top; `Everyone`/`All household` aggregation available in Library. Use a compact forest-green sidebar, task-centered screens, clear primary and quiet actions, accessible keyboard controls, and responsive layouts suited to a Windows desktop.

### Library (primary home)

- Search by title, author, ISBN, tags and text fields; instant-feeling search.
- Browse as compact rows by default; optional cover grid if local covers exist.
- Each row shows title, author, small relevant status indicators and fast actions (`Mark read`, `Log reading`, `Add to list`). Avoid requiring a full detail screen for common changes.
- Filters by person, reading status, list status, rating, custom fields, last read, and optional tags. Support Unknown separately from false, blank and zero.
- Sort by recently added, title, author, most recently read, rating and reading count where relevant.
- `Everyone` view shows books across household; shared metadata appears once, with per-person status indicators.
- A one-click `Add book` action opens an inline/modal quick-entry form requiring only a title. Optional advanced details can be expanded later.
- Show a welcoming empty state with `Add your first book`.

### Book detail

**Shared section:** title, subtitle, authors, illustrator (if used), ISBN-13, publisher, publication year, language, series/volume, description, cover (optional local attachment), tags, and book-scoped custom fields. All metadata optional except title.

**Person section:** select a household member, then show their reading status, rating, personal list status, notes, person-book custom fields, and history. Make it clear which changes affect just that person.

**Activity section:** quick `Log read today` plus optional date/time, participants, and note. Show reading events chronologically. Repeat reads are valid.

An individual can be attached to an already-known book from the detail view or through their library; adding a person does **not** automatically assign every book to them.

### Activity

- Show recent reading events, optionally filtered by person and date.
- Repeat-reading sessions can be logged for a toddler in a single action.
- Default date is current local date, editable for backfill.
- One event can include multiple participants (e.g. mother reading with toddler), so an individual session isn't accidentally counted twice in a household summary. A participant can be added/removed independently.
- Recent book shortcuts for rapid repeat logging.
- Provide straightforward edit/delete with confirmation for accidental entries.

### People

- Create a person with just a display name; optional color/avatar initials.
- Change active person from the global switcher, without login.
- Person names are labels, not security identities. All data remains accessible to anyone using the app/device.
- Preserve a stable person ID when renaming.
- Deactivate/archive a person without deleting history; hard delete requires explicit warning and backup.
- Distinguish `Everyone` (a filter) from a real person.

### Settings

- Manage custom fields by scope, label, type, options, default, order, and visibility.
- Manage people; export/import backups; data location and recovery.
- Optional personal preferences (list or grid view, default selected person, show covers), persisted locally.
- No account, network, analytics, or sharing settings.

## 4. Default attributes and behaviors

| Attribute | Scope | Type | Behavior |
|---|---|---|---|
| Read / reading status | Person-book | Built-in select | `Want to Read`, `On Hold`, `Currently Borrowed`, `Read`, plus optional `Reading` / `Did Not Finish`; default `Not set`. Quick `Mark read` available. |
| Rating | Person-book | Built-in select | `Loved`, `Liked`, `Meh`, `Nope` (matching imported workbook); no rating is `Unknown`. A numeric rating is an optional later feature, not a forced conversion. |
| List status | Person-book | Built-in select | `Library`, `Wishlist`, `Borrowed`, `Not set` for personal organization. |
| Notes | Person-book | Built-in multiline text | Personal impressions, toddler reactions, etc. |
| Owned | Household / copy | Built-in boolean or count | Track physical household ownership independently of personal wishlist; default unknown. |
| Recommend? | Person-book custom field | Boolean | Example add-on; Unknown/Yes/No, never silently No. |

**Important naming decision:** The phrase `owned/library/wish list` mixes possession and a person's intent. Do not force these into one mutually exclusive value. **Owned** is a household property; **Library/Wishlist/Borrowed** are personal list statuses. If the owner explicitly prefers a single field for all three, the UI can offer a simplified view while keeping the data meanings separate.

When a reading event is recorded, propose/set the relevant participant's status to `Read` if not already read, but do not erase existing notes, ratings or earlier events. Manual `Mark read` changes status without fabricating a dated event. Derive `last_read_at` and `read_count` from reading events; an optional manually entered historic completion date should be explicitly labeled, not silently treated as an event. Status `Read` alone does not imply a known reading date.

## 5. Custom attributes (core feature)

Use stable field definitions plus typed values, **not one new SQLite column per custom field** and not a giant opaque blob of all fields on each book.

### Supported scopes

- `book`: shared facts (e.g. illustrator, genre detail, age range).
- `person_book`: a person's relationship with a title (e.g. recommend, child enjoyed, bedtime favorite, favorite character).
- `person`: optional later extension for person preferences (not needed for MVP).

Scope is fixed once values exist unless an explicit migration is performed. MVP UI can present `Shared book attribute` vs `Per-person attribute` in plain language.

### Supported types

- Boolean: `Yes`, `No`, `Unknown` (unset/null).
- Select: one choice from owner-managed options with durable option IDs.
- Number: decimal, optional min/max, optional unit label.
- Date: ISO calendar date in storage, local date display.
- Text: short or multiline.
- Rating: configurable range, initially 1–5 if custom ratings are used.

### Field-definition behavior

- Each definition includes stable ID, scope, label, kind, description, optional default, select choices, display order, active flag, and settings relevant to validation.
- Creating a new definition makes it *available* across every existing and future record in that scope immediately. It need not create one physical value row per book.
- Null/missing value renders as `Not set`/`Unknown` unless a display-only default is configured. A default should not silently overwrite a person's choice or pretend missing values were deliberately set.
- The owner may choose whether a field appears in the book detail, on library rows, and/or among filters. Keep row displays compact; no automatic clutter from many fields.
- Field labels and select labels can be renamed without changing IDs or invalidating stored values.
- Duplicate active field labels in the same scope are rejected case-insensitively or clearly disambiguated.
- Change of type, deletion, and option removal require impact preview and explicit confirmation. Prefer deactivate/hide over destructive deletion; no implicit string-to-number conversions.
- Values are validated by the Rust backend based on field definitions; unknown field IDs and invalid select option IDs must be rejected.
- Custom fields contain data only: never treat their definitions or entered values as executable JS, SQL or template code.

### Example workflow

Settings → Custom fields → Add attribute → Name: `Recommend?` → Type: `Yes / No` → Applies to: `Each person's book` → Save. The new control then appears for any assigned person/book, initially `Unknown`, and can be included as a filter across existing records. **No schema migration or manual bulk update.**

## 6. Proposed technical architecture

```text
bookbuddy/
  AGENTS.md
  DESIGN.md
  PLAN.md
  docs/adr/
  src/                         # React + TypeScript
    app/                       # shell, routing, query client
    features/
      library/ books/ people/ activity/ custom-fields/ settings/
    components/                # shared UI
    lib/                       # typed Tauri invoke wrappers, formatting
  src-tauri/
    src/
      commands/                # narrow Tauri IPC surface
      domain/                  # entities, validators, custom-field type rules
      services/                # book creation, assignment, reading logs, filters
      persistence/             # SQLite repositories and transactions
      lib.rs
    migrations/
    capabilities/              # least-privilege Tauri permissions
  tests/
```

**Frontend:** React, TypeScript, Tailwind CSS, shadcn/ui or similar accessible components, TanStack Query for local async data queries. Frontend handles UI state; backend owns validation and invariants.

**Backend:** Rust with Tauri 2 typed commands; narrow domain functions for updating books, assigning people, validating dynamic field values, logging readings, and transactional import/export.

**Persistence:** SQLite + SQLx migrations; foreign keys on, WAL where appropriate, indexed search, transactional writes, backups. No server process, hosted database or telemetry. Use application-data directory, not the installer directory.

### Conceptual data model

| Entity | Key fields |
|---|---|
| `person` | id, display_name, color/avatar, archived_at, created_at |
| `book` | id, title, subtitle, isbn13 nullable, publisher, publication_year, language, description, cover_path nullable, timestamps |
| `book_author` | book_id, author_id, role, order_index (or ordered author text for a very lean MVP) |
| `person_book` | id, person_id, book_id, reading_status, rating, list_status, notes, timestamps; **UNIQUE(person_id, book_id)** |
| `household_copy` | id, book_id, format optional, quantity, notes; optional MVP simplification: an ownership state directly on book |
| `reading_event` | id, book_id, occurred_on, note, created_at |
| `reading_event_participant` | event_id, person_id; **UNIQUE(event_id, person_id)** |
| `custom_field_definition` | id, scope, label, kind, settings_json, order_index, active, timestamps |
| `book_custom_field_value` | book_id, field_id, value_json, updated_at; **PRIMARY KEY(book_id, field_id)** |
| `person_book_custom_field_value` | person_book_id, field_id, value_json, updated_at; **PRIMARY KEY(person_book_id, field_id)** |
| `book_tag` / `tag` | normalized tags and associations; optional MVP |
| `app_settings` | namespaced validated local preferences |

**Custom-value storage:** `value_json` is a small canonical, type-checked JSON scalar or object validated against `custom_field_definition` on every write, not arbitrarily trusted. SQL queries join the appropriate scope; indexes and explicit typed query projections can be introduced for frequently filtered fields. A more fully typed alternative is `value_boolean`, `value_text`, `value_number`, `value_date`, `option_id` columns with exactly one populated value. Either is acceptable if invariants are tested. Do not allow a book-scoped field ID to appear in person-book values or vice versa.

**Deletion semantics:** deleting a `person_book` does not delete the shared `book`. Deleting the shared book with references requires confirmation and an explicit cascade policy. A reading event may be retained for its historical participants even when a person's library relationship is archived. Soft archive by default for people and field definitions.

### Conceptual relationships

```text
book 1 ─── * person_book * ─── 1 person
  │               │
  │               └── * person_book_custom_field_value
  ├── * book_custom_field_value
  ├── * household_copy
  └── * reading_event 1 ─── * reading_event_participant * ─── 1 person

custom_field_definition 1 ─── * (values in the matching scope)
```

## 7. Application commands / contracts

Use typed Tauri commands rather than a generic SQL query endpoint:

- `list_books(filter, sort, person_id?)`, `get_book(book_id)`, `create_book(draft)`, `update_book(id, patch)`, `archive_or_delete_book(id)`.
- `list_people()`, `create_person(name)`, `rename_person(id, name)`, `archive_person(id)`.
- `ensure_person_book(person_id, book_id)`, `update_person_book(id, patch)`.
- `log_reading(book_id, occurred_on, participant_ids, note?)`, `edit_reading_event`, `delete_reading_event`, `list_reading_events(filters)`.
- `list_custom_field_definitions(scope?)`, `create_custom_field_definition`, `update_custom_field_definition`, `deactivate_custom_field_definition`, `set_custom_field_value(target, field_id, value)`.
- `export_backup(destination)`, `import_backup(source, review_mode)`.

Every write validates references, payload types, and field scope on the Rust side; use transactional operations where a change touches several tables.

## 8. Important business rules and edge cases

- Adding a book globally does not automatically add it to all people. The global catalog and personal libraries are distinct.
- One person can have at most one `person_book` per book; multiple reading events are expected.
- Read status and reading events are distinct: updating status is not equivalent to logging a date. UI offers both actions clearly.
- A shared reading session with two participants counts once as a household event and once toward each participating person's session count.
- No rating is stored as null; never infer 0/5.
- An unset boolean means Unknown, distinct from No.
- A person can wishlist a title already owned by the household; show this without conflict.
- Do not silently overwrite another person's notes when switching people. Prominently label the editing context.
- Titles may repeat (different editions or books with same title). ISBN-13 may be used as a duplicate hint, but is optional, and editions should not be silently merged.
- Deleting a field definition should not automatically discard potentially valuable values. Prefer hide/archive; deliberate permanent deletion includes count of affected values.
- A field definition's display default and a persisted actual value are different. Imports preserve the distinction.
- Imported data is untrusted; never execute HTML from descriptions/notes.
- Empty author, unknown publication year, books without ISBN and coverless books are valid.
- Person deletion/archive must not accidentally destroy globally shared book metadata.

## 9. Search and performance

- Search titles and authors without case sensitivity; optional SQLite FTS5 where practical for descriptions and notes. Provide exact behavior for diacritics/normalization.
- Filter on `person_book` built-in fields with indexed columns; filter custom fields via definition/value joins scoped appropriately.
- Include `Unknown` filters explicitly (values absent from the value table).
- In MVP, a few thousand titles, people and reading events should feel instantaneous on a typical Windows PC. Favor simple queries; optimize only after profiling.
- Add recent-book shortcuts for repetitive toddler sessions. No network is required for any core operation.

## 10. Backup, portability, security

- Store everything locally in SQLite, plus optional local cover assets. No user account or cloud requirement.
- Provide `Export backup` as a versioned archive containing JSON data and referenced local covers, or an SQLite snapshot plus a manifest. A backup must be restorable with the schema version checked before writing.
- Ensure consistent snapshot/export while database is active (SQLite backup API or an equivalent transaction-safe strategy); never simply copy a live WAL-backed database without its consistency requirements.
- Offer a manual export and a modest automatic local-backup rotation; document backup directory and encourage saving a copy outside the device.
- Import starts with validation and preview, saves a pre-restore backup, and performs an atomic replace/merge strategy. MVP can support **restore/replace only**, explicitly warning that merge is unsupported.
- All book data is local; anyone with OS access to the app data can read it. Multiple people are organizational profiles, not private accounts.
- Restrict filesystem access through narrow Tauri capabilities; validate inputs, run migrations safely, escape UI text.
- No analytics, background web traffic, update checker or online metadata fetch needed for the MVP.

## 11. Implementation roadmap overview

### Phase 1 — Foundation

Create Tauri/React project, SQLite migrations, basic shell/sidebar, settings, typed commands, local persistence and a person picker.

### Phase 2 — Books and personal libraries

Create/edit/search books, assign them to people, manage read status, rating, list status and notes. Provide fast inline updates and helpful empty states.

### Phase 3 — Custom attributes

Build definition editor, supported types, scoped value storage and validation, detail rendering, Unknown semantics and custom-field filtering. Test adding new fields after hundreds of books exist.

### Phase 4 — Reading activity

Log repeated reading events, multi-person participation, recent reads and derived counts/dates. Support backfill and editing errors.

### Phase 5 — XLSX library import and resilience

Implement the five-sheet workbook adapter, import preview and per-person mapping, borrowing-history provenance, title-verification queue, deduplication and idempotency tests. Include backup/rollback before committing changes.

### Phase 6 — Resilience and polish

Versioned backups, restore preview, archive safeguards, keyboard navigation, accessibility review, Windows installer and scripted smoke tests. Add only small quality-of-life refinements based on actual use.

### Later, only if genuinely useful

- ISBN lookup, barcode scanning using an attached camera, cover downloads: opt-in, network-dependent convenience, not prerequisites.
- Generic CSV import from other book lists, and bulk assignment of selected books to one person. The provided XLSX workbook import is part of MVP.
- A `series` browser, reading-goal summaries or a simple toddler favorites view.
- Per-person customizable column layouts.

## 12. Testing priorities

**Rust unit tests:** custom-field type/scope validation; Unknown vs false; select option rename; read status vs events; duplicate person-book prevention; event participation deduplication.

**SQLite integration tests:** migrations on old data; archives/deletions and foreign keys; creating fields after records exist; custom-field filtering including missing values; safe backup and restore round trip.

**End-to-end flows:** new household starts with one person; title-only book addition; mother logs toddler reading in two actions; adds herself and a separate rating; adds `Recommend?` field and filters older records; restarts app and sees identical data; exports, restores and verifies records.

## 13. MVP definition of done

On a fresh Windows install, a user can create a household person, add a title-only book, associate it with that person, mark it read or record multiple reading sessions, edit rating/list status/notes, create another person who can independently rate or track the same book, define a new boolean field `Recommend?` that applies to existing person-book records without a migration, filter using that field, restart without data loss, and import the supplied Carly library XLSX with separate historical loans and verification tasks, and successfully export and restore a local backup. All of this works without accounts, cloud services, or internet connectivity.

## 14. Architectural decisions to record

- **ADR 0001 — Separate shared book catalog from per-person book records.**
- **ADR 0002 — Custom field definitions and typed per-scope values instead of dynamic schema columns.**
- **ADR 0003 — Reading events are historical facts, independent of manual reading status.**
- **ADR 0004 — Household ownership differs from personal list membership.**
- **ADR 0005 — No identities, accounts, cloud, or network requirement; people are local labels.**
- **ADR 0006 — Versioned local backups and explicit safe restore.**
- **ADR 0007 — Workbook imports preserve provenance, historical date precision, and independent per-person data.**

## 15. XLSX import — Carly's Little Library workbook (MVP requirement)

### 15.1 Supported file and workbook structure

The first supported import source is the provided `Carly's Little Library import.xlsx` workbook. Import is **local and entirely offline**, using a Rust XLSX reader (e.g. calamine) and no remote services. The supplied sample contains these worksheets:

| Sheet | Observed structure | Meaning / treatment |
|---|---|---|
| `Book Catalogue` | Header row plus **312 book rows**, 20 columns `A:T` | Primary catalogue of unique books; source of metadata and existing statuses |
| `Library History` | Header row plus **253 history rows**, columns `A:G` | Borrowing/library history; **not** a record of reading sessions |
| `Check Titles` | Header row plus **35 verification rows**, columns `A:E` | Uncertain/clipped title strings with editable confirmed-title slot |
| `Dashboard` | Summaries and calculated totals | Ignore for data writes; optionally cross-check totals for warnings |
| `How to Use` | Explanation of field meaning | Reference for interpreting the source workbook; not imported as book records |

The loader should detect sheets by normalized names, detect header columns by normalized names rather than fixed positions, tolerate added columns, ignore blank rows, and report missing or ambiguous required headers. Do not treat dashboard formula values as authoritative source records. Support the exact supplied layout first, then harmless variations of it; unknown future workbook layouts must fail gracefully or offer a reviewed manual mapping instead of silently misinterpreting data.

### 15.2 `Book Catalogue` mappings

| Excel column | Destination | Import rule |
|---|---|---|
| `Title` | `book.title` | Required; preserve original wording, trim surrounding whitespace; missing titles are row errors |
| `Author` | `book` author(s) | Preserve source author string including `Surname, Given`; `??`/blank means unknown, not literal author identity |
| `Format` | Book-scope `format` attribute | Import `Book`, `Board Book`, `Hardcover Book`, `Paperback Book`, `Unknown` without discarding unfamiliar options |
| `Published` | `book.publication_year` | Validate plausible integer year; blank remains unknown |
| `Reading Status` | Carly's `person_book.reading_status` | Map workbook choices including `Read`, `Currently Borrowed`, `On Hold`, `Want to Read`; preserve raw unsupported choice for review |
| `Owned?` | Shared household ownership state | `Yes` = owned, `No` = not owned; blank = unknown. **Not owned does not mean wishlist.** |
| `Carly Rating` | Carly's `person_book.rating` | `Loved`, `Liked`, `Meh`, `Nope`, blank = unknown |
| `Mom Rating` | Mom's `person_book.rating` | Same four choices; create Mom's person-book record when rating exists; do not copy Carly's status to Mom |
| `Favourite?` | Carly's `person_book` favourite boolean | `Yes`/`No`/unknown; do not equate it to a numeric rating |
| `Buy Status` | Shared or household book acquisition attribute | `Purchased`, `Consider Buying`, `Want to Buy`, blank; does not override ownership or personal reading list |
| `Borrow Count` | Imported borrow-count baseline / reconciliation | Preserve supplied count and provenance; reconcile with matched history records, do not create synthetic dates |
| `First Library Date` | Library-summary source metadata | Store the source date and its precision/date-type, do not invent checkout date |
| `Latest Library Date` | Library-summary source metadata | Same; should not become `last_read_at` |
| `Date Type` | Provenance marker for dates | Preserve `Historical return date` or unknown raw value; never claim an actual checkout happened on it |
| `Reading Effort` | Carly-specific custom attribute | Preserve raw entry, allow user to edit field definition/options |
| `Age Range` | Book-scope custom attribute | Preserve as written; do not automatically infer recommended ages |
| `Renewed to Death?` | Carly-specific custom boolean attribute | `Yes` / `No` / unknown; historical absence remains unknown |
| `Notes` | Book/source notes | Preserve text and source; default to shared notes when no person association is explicit |
| `Source / Original Title` | Import provenance | Preserve exact source label/clipped title; never overwrite automatically |
| `Check Title?` | `needs_title_verification` | `Yes` queues book for review, including after initial import |

A household may later customize field labels and types without breaking stored values. For interoperability, built-in fields retain durable internal IDs; source-specific columns may be represented by preseeded, user-editable field definitions. The importer can create definitions when absent, with confirmation and a mapping preview.

### 15.3 People, borrowing events and title verification

- On first import, offer to create/select people named **Carly** and **Mom**. Their names are editable local labels; they are not accounts. The catalogue's `Reading Status` belongs to Carly; a `Mom Rating` alone must not imply Mom read the book unless explicitly provided.
- Create Carly `person_book` entries for the catalogue; do not manufacture Mom entries for all titles simply because a `Mom Rating` column exists.
- `Library History` rows contain `Title`, `Author`, `Format`, `Publication Year`, `Approximate Borrow Date`, `Actual Date Type`, `Source PDF`. Match to catalogue entries using normalized title+author with an explicit review path for ambiguous or unmatched rows. Each history row becomes a **library_borrow_event / import historical record**, not a `reading_event`. Preserve the `Actual Date Type` explanation and `Source PDF` provenance.
- Dates from old library PDFs represent **historical return dates used as approximate checkout dates**, not actual checkout dates. The app must retain `source_date`, `date_interpretation`, and `date_precision`, distinguish these from confirmed future checkout/return dates, and render an explanatory label. The workbook itself does not establish whether every borrowed book was actually read.
- Import the `Check Titles` sheet: `Working Title`, `Author`, `Exactly as PDF shows`, `Reason`, `Confirmed Full Title (edit here)`. Attach the verification task to the best-matched book or keep it as an unresolved review item. Never silently replace a working title using fuzzy matching; only use a confirmed-title value after showing the change in preview.
- Treat `Borrow Count` as a source baseline even if the matched history count differs. Surface mismatch in the preview; never silently discard the catalogue count or invent missing history rows. Keep future loans separate from imported totals so derived totals cannot double-count history.
- The `How to Use` sheet says six personal YA loans from 2019 and one DVD were deliberately excluded. Do not attempt to re-add exclusions that are not present in this workbook.

### 15.4 UX: import wizard

Settings → Data → **Import Excel library** (also visible as a first-run option):

1. **Choose file**: native local `.xlsx` picker; never upload workbook contents.
2. **Inspect**: show detected worksheets, row counts, proposed person names, inferred field mappings, and source warnings. Importing does not modify the database at this stage.
3. **Review**: show new vs matched books, verification tasks, ambiguous title/author matches, invalid dates, unsupported field values, and borrow-count differences. Allow assignment to existing people and adjustments to field mapping.
4. **Confirm**: pre-import SQLite backup, transactional import, rollback on any fatal error, and visible summary: books added/updated/skipped, person records, historical loans, fields, unresolved reviews.
5. **After import**: navigate to Library with Carly selected; expose a **Needs title verification** filter and an Import History screen containing source rows and warnings.

### 15.5 Duplicate handling / repeat import

- Compute import fingerprints per source worksheet and row (using normalized stable source values and a workbook import ID). Persist raw source row payload and mapping version for traceability.
- A repeat of an identical workbook must be **idempotent**: no doubled books, person-book records, or library-history entries.
- Match confirmed ISBN when available, otherwise normalized title and author with caution; do not conflate different editions or similarly titled books purely on fuzzy similarity.
- For an existing book, show whether incoming values will fill blanks, overwrite changed data, or be skipped. **Default: preserve newer user edits**; imported blank cells never erase existing values. Allow explicit overwrite with preview.
- Imported history events have stable deduplication keys based on source identity/row occurrence; repeated identical historical borrow entries can legitimately exist and must not be collapsed merely for sharing title/date. Reimporting the same row should not duplicate it.
- Preserve ignored/unmapped nonempty columns as reviewed source metadata or surface a warning; never silently throw away values.

### 15.6 Additional persistence and API contracts

Add the following entities (or equivalent normalized tables) to the conceptual model:

| Entity | Key fields |
|---|---|
| `library_borrow_event` | id, book_id, person_id, source_date, date_interpretation, date_precision, source_pdf, imported_row_id, notes, timestamps |
| `book_title_verification` | id, book_id nullable, working_title, raw_pdf_title, reason, proposed_confirmed_title, state, source_row_id |
| `import_batch` | id, source_filename, source_hash, mapping_version, imported_at, summary_json |
| `import_source_row` | id, batch_id, sheet_name, sheet_row_number, row_fingerprint, raw_json, linked_entity_type/id, warning_json |
| `book_borrow_summary` | book_id, imported_count_baseline, first_source_date, latest_source_date, source_date_type, updated_at; optional if derived equivalently |

Typed commands: `inspect_library_xlsx(path) -> ImportPreview`, `preview_library_xlsx_mapping(draft) -> ImportPreview`, `commit_library_xlsx_import(reviewed_plan) -> ImportSummary`, `list_title_verification_tasks()`, `resolve_title_verification(id, decision)`. Revalidate the source file's hash between inspection and commit to avoid a changed-file time-of-check/time-of-use issue. Keep file access constrained to native chosen paths and narrowly scoped Tauri permissions.

The desktop may combine choose and inspect in `choose_library_xlsx()` and use an opaque, process-local selection token for subsequent preview/commit instead of accepting arbitrary frontend paths (ADR 0007). Unmatched historical borrowing rows may remain unlinked (`book_id` nullable) with their complete source evidence until explicitly reviewed; this never creates a guessed catalog identity. Missing Library History or Check Titles requires acknowledgement of the incomplete source; Book Catalogue and its Title mapping remain essential.

### 15.7 Acceptance criteria — sample workbook

- Recognizes all five sheets in the supplied source workbook, imports **312 catalogue rows**, **253 library-history rows**, and **35 title-verification rows** without falsely importing dashboard summaries as books.
- Maintains **66 owned** items and **3 currently borrowed** statuses as represented in the source. The observed sample has **37 favourite** flags, and both rating columns are currently blank; all missing ratings remain unknown.
- Imported library history **does not generate reading events**, and borrowed/return-proxy dates do not appear as actual reading dates.
- A second import of the same unchanged file adds no duplicates. An import with corrected verified titles shows merge proposals and preserves user changes by default.
- Handles an incomplete/missing worksheet, a duplicate title, unknown author `??`, blank numeric/year/date cells, Excel serial date values, and invalid yes/no or option strings without data corruption.
- Supports all other core offline functionality even if Excel import fails.

## 16. Initial decisions and defaults

- Application name: **BookBuddy** (approved).
- Platform and stack: Windows desktop + Tauri 2 / Rust / React / TypeScript / SQLite.
- First launch: prompt for one person's display name; no account setup.
- Initial active view: first person, Library in compact list mode.
- Default built-in personal attributes: reading status, categorical rating (`Loved`/`Liked`/`Meh`/`Nope`), list status, notes. XLSX import is supported in MVP.
- Default shared ownership: `Unknown`; users can explicitly mark owned or not owned.
- Example optional custom field: `Recommend?` (not forced on every book until added).
- Author and ISBN optional; title required.
- Covers optional; an attractive typographic placeholder is sufficient.
- Reading event dates use local calendar dates unless explicit times are later added.


## 17. Repository, build and operational boundaries

- `DESIGN.md` defines product semantics and architectural invariants. `PLAN.md` is the ordered implementation checklist, and `AGENTS.md` is mandatory working guidance for coding agents.
- Keep all application functionality usable offline. The XLSX adapter reads local files only. Do not add telemetry, cloud services, accounts, automatic network lookups, or an updater without an explicit design change.
- Rust owns SQLite access, transaction boundaries, validation, import mapping, deduplication and backup/restore. React owns presentation and form state; TypeScript types mirror serialized command contracts.
- Any file-system access must be constrained through explicit Tauri 2 capabilities and a user-selected native file dialog or application data directory. Remote page content must never receive privileged commands.
- Prefer additive, tested SQLx migrations; never modify a released migration. Ensure `PRAGMA foreign_keys=ON` on all connections and use robust local-path handling on Windows.
- On every release, preserve an upgrade path from an older local database and test backup/restore. Provide a documented data folder location in the app.
- Use short ADRs for material decisions that change book/person scoping, import interpretation, deduplication, history semantics, or backup format. Unresolved uncertainty is surfaced, not hidden by a guessed inference.

## 18. Acceptance tests and traceable fixtures

Keep a sanitized or expressly authorized fixture representing the five-sheet source shape, plus small synthetic workbooks for unusual values. The real household workbook is private test input; do not commit personal data or library exports into a public repository by default.

The workbook adapter tests must cover: header normalization and reordering; unknown columns; empty values; blank ratings; three-valued booleans; Excel dates; source date interpretation; repeated identical-looking library-loan rows; two user mappings; title verification; borrow-count mismatches; changed-file hash between preview and commit; interrupted transaction; identical reimport; edited reimport preserving manual changes; and missing/renamed sheets.

**Completion gates:** (1) household members have independent data on the same shared book; (2) `Recommend?` becomes queryable on old and new person-book records without a schema change; (3) borrowed history never fabricates reading events; (4) the representative XLSX import is loss-aware, previewable, atomic and repeatable; (5) backup/restore round-trips every relevant entity; and (6) all primary flows work offline after restarting the application.
