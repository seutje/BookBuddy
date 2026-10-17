# ADR 0003: Reading events are independent facts

Status: Accepted · 2026-10-17

Manual reading status changes create no dated event. An explicitly logged event
uses a local calendar date and a unique set of participants. Repeated readings
are legitimate separate events. Counts and last-read dates derive from events.

One shared session counts once for the household and once for each participant.
Historical library borrowing remains separate and never fabricates reading.

Reference: DESIGN §§4, 6, 8, 15.

Phase 4 behavior: logging proposes setting active participants to Read through an
explicit form checkbox (enabled by default). The one-click "Log read today" action
sets Read and records the Windows local calendar date; ratings, lists and notes
remain unchanged. Editing and deleting never downgrade manual status. Future dates
are rejected; timestamps and UTC conversions are not used to assign reading dates.
If Windows cannot supply a local date, logging stops with an actionable error.

Archived participants may be retained or removed from their existing events, but
cannot be added to new events. Each active participant is explicitly associated with
the book when logging. Editing keeps the book fixed; correction to a different book
requires deleting the mistaken event and logging on the intended book. Event deletion
cascades only its participant links, and the UI requires confirmation.
