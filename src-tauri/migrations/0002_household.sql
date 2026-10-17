CREATE TABLE person (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 100),
    archived_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE book (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    subtitle TEXT,
    authors TEXT,
    isbn13 TEXT,
    publisher TEXT,
    publication_year INTEGER,
    language TEXT,
    series TEXT,
    volume TEXT,
    description TEXT,
    notes TEXT,
    cover_path TEXT,
    owned INTEGER CHECK (owned IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE person_book (
    id TEXT PRIMARY KEY NOT NULL,
    person_id TEXT NOT NULL REFERENCES person(id) ON DELETE RESTRICT,
    book_id TEXT NOT NULL REFERENCES book(id) ON DELETE RESTRICT,
    reading_status TEXT CHECK (reading_status IN ('want_to_read', 'on_hold', 'currently_borrowed', 'read', 'reading', 'did_not_finish')),
    rating TEXT CHECK (rating IN ('loved', 'liked', 'meh', 'nope')),
    list_status TEXT CHECK (list_status IN ('library', 'wishlist', 'borrowed')),
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (person_id, book_id)
);
CREATE INDEX person_book_book ON person_book(book_id);
CREATE INDEX person_book_status ON person_book(person_id, reading_status, list_status, rating);

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL CHECK (json_valid(value_json))
);
