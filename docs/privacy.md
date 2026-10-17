# Local data and privacy

BookBuddy stores the household in SQLite on this Windows computer. It has no
accounts, authentication, hosted storage, telemetry, analytics, cloud sync,
metadata requests or automatic cover downloads. All core
features work without internet.

People organize independent statuses, ratings, notes and reading participation.
They are not access-controlled identities. Anyone with access to this Windows user
and its data folder can read the household library. Use Windows account/device
protection appropriate for the computer.

The app-data location is shown in Settings. Release data normally lives under
`%LOCALAPPDATA%\local.bookbuddy.desktop`; development/testing uses separate folders.
The installer directory is separate from the data folder. Reinstalling/upgrading
the app preserves its database. Close the app before manually handling data files.

Imports and backups are local files. Original workbook cells, notes, source names,
PDF labels and provenance are retained in the library and backups. Portable backups
are unencrypted, so store/share them carefully. No private household workbook or
backup belongs in the public source repository. Database errors returned to the UI
omit raw rows, private names and paths.

The Windows installer bundles the app's assets and skips WebView2 downloading.
Microsoft's WebView2 Runtime is an existing platform prerequisite; obtain its
offline installer separately when needed. BookBuddy does not contact a metadata
or cloud service on startup or while using the library.

Installed releases check GitHub once at startup for a signed BookBuddy update;
Settings can retry. GitHub receives ordinary connection metadata, including your
IP address, when checking or downloading a release. Household
records, notes, names, workbook contents, local paths and backups are never sent.
Offline failures do not interrupt the library. Downloads and installation occur
only after confirmation, with a complete local recovery backup saved first.
