CREATE TABLE reading_event (
    id TEXT PRIMARY KEY NOT NULL,
    book_id TEXT NOT NULL REFERENCES book(id) ON DELETE RESTRICT,
    occurred_on TEXT NOT NULL CHECK (length(occurred_on) = 10),
    note TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX reading_event_book_date ON reading_event(book_id, occurred_on DESC);
CREATE INDEX reading_event_date ON reading_event(occurred_on DESC);
CREATE TABLE reading_event_participant (
    event_id TEXT NOT NULL REFERENCES reading_event(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES person(id) ON DELETE RESTRICT,
    PRIMARY KEY (event_id, person_id)
);
CREATE INDEX reading_participant_person ON reading_event_participant(person_id, event_id);
