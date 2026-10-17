# ADR 0003: Reading events are independent facts

Status: Accepted · 2026-10-17

Manual reading status changes create no dated event. An explicitly logged event
uses a local calendar date and a unique set of participants. Repeated readings
are legitimate separate events. Counts and last-read dates derive from events.

One shared session counts once for the household and once for each participant.
Historical library borrowing remains separate and never fabricates reading.

Reference: DESIGN §§4, 6, 8, 15.
