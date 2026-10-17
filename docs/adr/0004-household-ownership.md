# ADR 0004: Household ownership and personal lists

Status: Accepted · 2026-10-17

For MVP, store nullable boolean ownership on the shared book: Unknown, Yes or No.
Personal Library, Wishlist and Borrowed membership belongs to `person_book` and
is independent of ownership. No ownership value implies a personal list.

This simple representation preserves the distinction without requiring physical
copy inventory. Copy counts can be introduced through a later additive migration.

Reference: DESIGN §§2, 4, 6.
