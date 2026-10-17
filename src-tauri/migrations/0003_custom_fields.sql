CREATE TABLE custom_field_definition (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('book', 'person_book')),
    label TEXT NOT NULL CHECK (length(trim(label)) BETWEEN 1 AND 100),
    normalized_label TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('boolean', 'select', 'number', 'date', 'text', 'rating')),
    description TEXT NOT NULL,
    settings_json TEXT NOT NULL CHECK (json_valid(settings_json)),
    default_json TEXT CHECK (default_json IS NULL OR json_valid(default_json)),
    order_index INTEGER NOT NULL CHECK (order_index >= 0),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    show_detail INTEGER NOT NULL CHECK (show_detail IN (0, 1)),
    show_in_rows INTEGER NOT NULL CHECK (show_in_rows IN (0, 1)),
    show_in_filters INTEGER NOT NULL CHECK (show_in_filters IN (0, 1)),
    revision INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX custom_field_active_label ON custom_field_definition(scope, normalized_label) WHERE active = 1;

CREATE TABLE custom_field_option (
    id TEXT PRIMARY KEY NOT NULL,
    field_id TEXT NOT NULL REFERENCES custom_field_definition(id) ON DELETE RESTRICT,
    label TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1))
);
CREATE INDEX custom_option_field ON custom_field_option(field_id, order_index);

CREATE TABLE book_custom_field_value (
    book_id TEXT NOT NULL REFERENCES book(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES custom_field_definition(id) ON DELETE RESTRICT,
    value_json TEXT NOT NULL CHECK (json_valid(value_json) AND json_type(value_json) IN ('true', 'false', 'text', 'integer', 'real')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (book_id, field_id)
);
CREATE INDEX book_custom_field_lookup ON book_custom_field_value(field_id, value_json, book_id);

CREATE TABLE person_book_custom_field_value (
    person_book_id TEXT NOT NULL REFERENCES person_book(id) ON DELETE RESTRICT,
    field_id TEXT NOT NULL REFERENCES custom_field_definition(id) ON DELETE RESTRICT,
    value_json TEXT NOT NULL CHECK (json_valid(value_json) AND json_type(value_json) IN ('true', 'false', 'text', 'integer', 'real')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (person_book_id, field_id)
);
CREATE INDEX personal_custom_field_lookup ON person_book_custom_field_value(field_id, value_json, person_book_id);

CREATE TRIGGER book_custom_scope_insert BEFORE INSERT ON book_custom_field_value
WHEN (SELECT scope FROM custom_field_definition WHERE id = NEW.field_id) != 'book'
BEGIN SELECT RAISE(ABORT, 'custom field scope mismatch'); END;
CREATE TRIGGER book_custom_scope_update BEFORE UPDATE ON book_custom_field_value
WHEN (SELECT scope FROM custom_field_definition WHERE id = NEW.field_id) != 'book'
BEGIN SELECT RAISE(ABORT, 'custom field scope mismatch'); END;
CREATE TRIGGER personal_custom_scope_insert BEFORE INSERT ON person_book_custom_field_value
WHEN (SELECT scope FROM custom_field_definition WHERE id = NEW.field_id) != 'person_book'
BEGIN SELECT RAISE(ABORT, 'custom field scope mismatch'); END;
CREATE TRIGGER personal_custom_scope_update BEFORE UPDATE ON person_book_custom_field_value
WHEN (SELECT scope FROM custom_field_definition WHERE id = NEW.field_id) != 'person_book'
BEGIN SELECT RAISE(ABORT, 'custom field scope mismatch'); END;
CREATE TRIGGER definition_scope_update BEFORE UPDATE OF scope ON custom_field_definition
WHEN NEW.scope != OLD.scope AND (EXISTS (SELECT 1 FROM book_custom_field_value WHERE field_id = OLD.id) OR EXISTS (SELECT 1 FROM person_book_custom_field_value WHERE field_id = OLD.id))
BEGIN SELECT RAISE(ABORT, 'custom field scope is fixed while values exist'); END;
