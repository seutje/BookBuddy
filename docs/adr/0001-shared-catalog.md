# ADR 0001: Shared catalog and personal memberships

Status: Accepted · 2026-10-17

Books describe shared bibliographic identity. A separate `person_book` holds each
person's status, categorical rating, list status and notes, with a unique
`(person_id, book_id)` constraint. Adding a book or a person never assigns books
to everyone. Title similarity is a duplicate hint, never authority to merge.

This preserves independent household reading information without duplicating
editions. Person archival must preserve shared books and historical activity.

Reference: DESIGN §§2, 6, 8.
