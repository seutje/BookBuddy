# ADR 0010: Signed Windows updates with local recovery

Status: Accepted · 2026-10-17

The owner requested PantryPal-style automatic update discovery and a different
secret key. This explicitly extends DESIGN §17 and supersedes ADR 0005's exclusion
of an update service. All book, import and backup operations continue to work offline.

Use the maintained Tauri Rust updater SDK with signed-version verification and a
dedicated BookBuddy key. Commit only its public key. Release builds check once per
launch against `https://github.com/seutje/BookBuddy/releases/latest/download/latest.json`.
Startup errors are silent; Settings supports a deliberate retry. Debug builds,
browser previews and UUID-isolated acceptance launches never query production.

React displays plain-text notes, download progress and explicit installation consent.
Not now/Escape postpones without downloading. A cached startup promise survives
StrictMode replay. The native modal blocks editing during installation; users must
postpone and save any unsaved drafts before accepting.

Grant only two narrow application commands, retaining the reviewed update in Rust.
The UI cannot supply an endpoint, installer URL or path. No updater plugin IPC is
granted. Require credential-free HTTPS BookBuddy GitHub `.exe` release assets;
limit redirects to five and fixed GitHub/CDN hosts. Disable proxy discovery and set
15-second check/120-second download deadlines, a 128 MiB download limit and bounded
release-note validation. As with PantryPal, manifest/notes size validation happens
after SDK parsing rather than being a streaming response-memory limit.

The SDK verifies the signature and signed version before BookBuddy creates a complete
portable `pre-update-<UUID>.bookbuddy` backup. Backups use the existing WAL-consistent
export and are excluded from automatic rotation. A failed verification or backup
prevents installer handoff. Hold the operation write lock through installation to
exclude concurrent writes/restores. Keep SQLite open so a failed handoff leaves the
running library usable. The SDK exits after launching the per-user NSIS installer;
passive mode upgrades in place and restarts BookBuddy.

Tag builds publish the installer, checksum, `.exe.sig` and `latest.json` together.
Published assets and tags remain immutable (ADR 0009). Version 0.1.1 has no updater:
users must manually install the first updater-enabled version. Future versions
must retain this public key. Updater signatures are separate from Authenticode;
Windows publisher signing remains unconfigured.

Embed an explicit Windows `asInvoker` manifest in the app, native tests and
acceptance helper so an update-related product name cannot trigger legacy
installer elevation heuristics. Retain Common Controls v6 for native dialogs.

Tests use only synthetic data, signed payloads and owned loopback servers. Test-only
contexts may enable loopback HTTP; production configuration never does. A separate
installer identity is required for actual handoff tests, preserving household data.

References: [Tauri updater](https://v2.tauri.app/plugin/updater/), DESIGN §17.
