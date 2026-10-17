# Verification record

All test workbooks and databases are synthetic or disposable. The supplied private
workbook is used only by explicitly invoked aggregate tests; its rows are never
printed or committed. Household app data is not opened by desktop acceptance tests.

## Local performance

The opt-in `performance` Rust integration test seeds 5,000 books/memberships and
20,000 actual reading sessions/participants in a temporary WAL database. The
original correlated membership-total query exceeded a minute and was stopped.
Replacing repeated reading scans with one grouped aggregate produced debug-build
medians around 144 ms for browse, 155 ms for literal search and 122 ms for scoped
ownership/count sorting on this Windows machine. Fetching 20,000 activity entries
fell from 879 ms to 219 ms after batching participant retrieval. These are local
measurements, not cross-machine guarantees.

Library and Activity render at most 100 rows per page; global search/filter/sort
still considers every matching record. A 1,001-book/event Windows fixture checks
next-page navigation, bounded rows and page reset after changing search.

## Accessibility and security

The keyboard script uses actual Tab/Shift+Tab, Enter/Space/Escape and native-choice
keys across Library, People, Activity, custom attributes, the native workbook
picker and confirmed import. A one-time focus entry activates its test window;
subsequent navigation uses keys. New editors focus their first field; the person
picker restores focus on Escape; main navigation moves focus to content.

Measured text contrast: body 11.39:1, muted text 6.07:1, primary buttons 9.52:1,
sidebar 10.75:1; focus outline on the main background 3.62:1. Controls have labels,
native keyboard semantics and visible focus indication.

Security tests compare the entire Tauri command permission list with explicit
command declarations, reject broad plugin permissions and remote capabilities,
check CSP and frontend execution/network sinks, and verify literal external strings
and invalid IDs. The keyboard test saves an HTML-shaped title and verifies its
literal accessible text. Archive/path/schema attacks and stale restore decisions
are exercised in backup tests. There is no generic SQL/filesystem IPC.

The final phase's exact check and installer results are recorded in PLAN.md after
they pass. The release requires Windows x64 and an existing WebView2 Runtime.

## Final release 0.1.1

`npm run check` passed with 50 regular Rust tests, plus all formatting, lint,
TypeScript, frontend and Clippy checks. The private preview/commit aggregate tests
passed explicitly. The final installer passed fresh install, an actual
0.1.0-to-0.1.1 upgrade and same-version reinstall, preserving the synthetic release
library. On Windows x64 10.0.26200.0, the installed final artifact passed the
foreground keyboard walkthrough and all eight remaining offline UI Automation
scripts (`offline-acceptance.ps1 -SkipKeyboard`). This included native XLSX and
backup pickers, import/reimport, restored reading counts, restart and large-list
pagination. The keyboard host initially refused foreground input; it passed when
the user made the desktop available. The script now checks foreground ownership
before sending each key, and stops rather than typing into another application.

Final installer SHA-256:
`FC50BBC6F9CEC5F8C3854C84C692EE1A785D7AACE423212EF4B99560501C329A`.
The installer is unsigned and uses the existing WebView2 Runtime. No other OS,
CPU architecture or clean-machine WebView installation was tested.
