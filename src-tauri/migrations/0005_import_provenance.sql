ALTER TABLE book ADD COLUMN needs_title_verification INTEGER NOT NULL DEFAULT 0 CHECK (needs_title_verification IN (0, 1));
CREATE INDEX book_needs_title_review ON book(needs_title_verification) WHERE needs_title_verification = 1;

CREATE TABLE import_source (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    last_filename TEXT NOT NULL,
    carly_person_id TEXT NOT NULL REFERENCES person(id) ON DELETE RESTRICT,
    mom_person_id TEXT REFERENCES person(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE import_batch (
    id TEXT PRIMARY KEY NOT NULL,
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_filename TEXT NOT NULL,
    source_hash TEXT NOT NULL UNIQUE,
    mapping_version INTEGER NOT NULL,
    imported_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    summary_json TEXT NOT NULL CHECK (json_valid(summary_json)),
    review_json TEXT NOT NULL CHECK (json_valid(review_json)),
    warnings_json TEXT NOT NULL CHECK (json_valid(warnings_json)),
    reference_json TEXT NOT NULL CHECK (json_valid(reference_json))
);
CREATE TABLE import_source_row (
    id TEXT PRIMARY KEY NOT NULL,
    batch_id TEXT NOT NULL REFERENCES import_batch(id) ON DELETE RESTRICT,
    sheet_name TEXT NOT NULL,
    sheet_row_number INTEGER NOT NULL,
    source_row_key TEXT NOT NULL,
    row_fingerprint TEXT NOT NULL,
    occurrence INTEGER NOT NULL CHECK (occurrence > 0),
    raw_json TEXT NOT NULL CHECK (json_valid(raw_json)),
    warning_json TEXT NOT NULL CHECK (json_valid(warning_json)),
    book_id TEXT REFERENCES book(id) ON DELETE RESTRICT,
    loan_id TEXT REFERENCES library_borrow_event(id) ON DELETE RESTRICT,
    verification_id TEXT REFERENCES book_title_verification(id) ON DELETE RESTRICT,
    CHECK ((book_id IS NOT NULL) + (loan_id IS NOT NULL) + (verification_id IS NOT NULL) <= 1),
    UNIQUE(batch_id, source_row_key),
    UNIQUE(batch_id, sheet_name, sheet_row_number)
);
CREATE TABLE library_borrow_event (
    id TEXT PRIMARY KEY NOT NULL,
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_row_key TEXT NOT NULL,
    book_id TEXT REFERENCES book(id) ON DELETE RESTRICT,
    person_id TEXT NOT NULL REFERENCES person(id) ON DELETE RESTRICT,
    source_title TEXT NOT NULL,
    source_author TEXT,
    source_format TEXT,
    publication_year INTEGER,
    source_date_json TEXT NOT NULL CHECK (json_valid(source_date_json)),
    date_interpretation TEXT NOT NULL,
    date_precision TEXT NOT NULL,
    source_pdf TEXT,
    imported_row_id TEXT NOT NULL REFERENCES import_source_row(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(origin_id, source_row_key)
);
CREATE INDEX library_borrow_book ON library_borrow_event(book_id);
CREATE INDEX library_borrow_person ON library_borrow_event(person_id);
CREATE TABLE book_title_verification (
    id TEXT PRIMARY KEY NOT NULL,
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_row_key TEXT NOT NULL,
    book_id TEXT REFERENCES book(id) ON DELETE RESTRICT,
    working_title TEXT NOT NULL,
    raw_pdf_title TEXT,
    reason TEXT,
    proposed_confirmed_title TEXT,
    state TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'confirmed', 'corrected')),
    source_row_id TEXT NOT NULL REFERENCES import_source_row(id) ON DELETE RESTRICT,
    resolved_at TEXT,
    UNIQUE(origin_id, source_row_key)
);
CREATE INDEX title_verification_book_state ON book_title_verification(book_id, state);
CREATE TABLE book_borrow_summary (
    book_id TEXT PRIMARY KEY NOT NULL REFERENCES book(id) ON DELETE RESTRICT,
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    imported_count_baseline INTEGER CHECK (imported_count_baseline >= 0),
    first_source_date_json TEXT NOT NULL CHECK (json_valid(first_source_date_json)),
    latest_source_date_json TEXT NOT NULL CHECK (json_valid(latest_source_date_json)),
    source_date_type TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE import_field_mapping (
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_key TEXT NOT NULL,
    field_id TEXT NOT NULL REFERENCES custom_field_definition(id) ON DELETE RESTRICT,
    PRIMARY KEY(origin_id, source_key)
);
CREATE TABLE import_catalogue_identity (
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_row_key TEXT NOT NULL,
    book_id TEXT REFERENCES book(id) ON DELETE RESTRICT,
    skipped INTEGER NOT NULL CHECK (skipped IN (0, 1)),
    original_title TEXT,
    source_title TEXT NOT NULL,
    source_author TEXT,
    source_row_number INTEGER NOT NULL,
    batch_id TEXT NOT NULL REFERENCES import_batch(id) ON DELETE RESTRICT,
    CHECK ((skipped = 1 AND book_id IS NULL) OR (skipped = 0 AND book_id IS NOT NULL)),
    PRIMARY KEY(origin_id, source_row_key)
);
CREATE UNIQUE INDEX custom_option_identity ON custom_field_option(field_id, id);
CREATE TABLE import_option_mapping (
    origin_id TEXT NOT NULL REFERENCES import_source(id) ON DELETE RESTRICT,
    source_key TEXT NOT NULL,
    field_id TEXT NOT NULL REFERENCES custom_field_definition(id) ON DELETE RESTRICT,
    source_label_key TEXT NOT NULL,
    option_id TEXT NOT NULL,
    FOREIGN KEY(field_id, option_id) REFERENCES custom_field_option(field_id, id) ON DELETE RESTRICT,
    PRIMARY KEY(origin_id, source_key, field_id, source_label_key)
);
