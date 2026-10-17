# Windows release and acceptance

## Publish from a version tag

`.github/workflows/release.yml` automatically checks, builds and publishes the
Windows x64 NSIS installer when a stable version tag such as `v0.1.2` is pushed.
The workflow must be present in the tagged commit. Before tagging, keep the
version in `package.json`, `package-lock.json` (including its root package),
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`
in sync. The workflow rejects mismatches and noncanonical/prerelease tags.

```powershell
git tag v0.1.2
git push origin v0.1.2
```

Every push to `main` also checks whether its current package version has an
existing, unpublished tag in the branch history. If so, it recovers a missed tag
event automatically. The installer always builds from that tag's resolved commit;
the recovery never moves or recreates a tag. Ordinary untagged development versions
and already published releases skip the Windows build.

To retry explicitly, open Actions → Publish Windows release → Run workflow on
`main`, and enter the existing tag (for example `v0.1.1`). Published release assets
are preserved; an unfinished draft can resume uploading and publishing.

The Windows build job runs `npm ci` and `npm run check`, then packages with the
Cargo lockfile enforced. The installer, SHA-256 sidecar, signature and feed pass to the
publishing job, which creates a published GitHub Release with generated notes using the
built-in `GITHUB_TOKEN` and `contents: write` permission. Signing additionally
requires the dedicated key secret described below. Existing releases are not
overwritten on rerun. Private-workbook tests
and interactive desktop smoke tests are not run on GitHub; perform the documented
local release acceptance before tagging. No tag is pushed by the workflow itself.

## Signed application updates

Starting with 0.1.2, installed releases check once on startup and offer plain-text
notes, Not now and Download and install. Settings → Application updates retries
manually. Offline startup failures are silent. Save any edits before accepting:
BookBuddy verifies the signed version and installer, saves a complete portable
backup under `backups/pre-update-<UUID>.bookbuddy`, then closes for the passive
Windows installer to upgrade and restart it. Failure before handoff leaves the
library usable. Recovery backups are excluded from automatic rotation.

Version 0.1.1 has no updater; install 0.1.2 manually once to gain future updates.
Development, browser preview and isolated acceptance launches skip production checks.

The dedicated BookBuddy private key is held only in the ignored, user-restricted
`.local/release-secrets/bookbuddy.key`. Its public key is committed in Tauri config;
it differs from PantryPal's key. Keep a secure offline copy: replacing or losing
this key prevents updates from being accepted by existing installations.

Set the BookBuddy repository Actions secret `TAURI_SIGNING_PRIVATE_KEY` to that
file's contents. The generated key has an empty password; no password secret is
needed. Authenticated GitHub CLI users can run
`powershell -File scripts/configure-release-signing.ps1` to upload it via stdin.
No private key is printed or committed. CI fails if its signing secret is missing.

The workflow publishes `.exe`, `.exe.sha256`, `.exe.sig` and `latest.json` together.
The fixed public feed requires no token in the application. Signatures bind the
version as well as installer bytes; relabelled or altered downloads are rejected.
This is updater signing, separate from a Windows Authenticode publisher certificate.

Run `powershell -File scripts/smoke-signed-upgrade.ps1` for the separately identified
synthetic install/upgrade/restart/uninstall acceptance. It refuses running BookBuddy
processes and preserves its local evidence and data. After publication, verify the
actual public feed and installer signature without executing it:

```powershell
. ./scripts/windows-toolchain.ps1
cargo test --manifest-path src-tauri/Cargo.toml --locked --test signed_releases published_feed_verifies -- --ignored
```

## Build and test locally

Build on Windows x64 with Node/npm, Rust MSVC, Visual Studio C++ tools and WebView2.
Use `npm ci`, the committed Cargo lockfile, and `npm run check` before packaging.
`scripts/windows-toolchain.ps1` imports the installed C++ compiler environment.

```powershell
powershell -File scripts/release-build.ps1
```

The installer is generated at
`src-tauri/target/release/bundle/nsis/BookBuddy_0.1.2_x64-setup.exe`.
It installs for the current Windows user, with bundled application assets, and
does not download WebView2. The Evergreen Runtime must already be available.
The release is unsigned; no signing certificate is configured in this repository.
The first build may obtain packaging tools in Tauri's user cache. That build-time
dependency is separate from the installed app's offline operation.

The normal dataset stays in app data, separate from installation files. Updating
or reinstalling preserves it. In the uninstaller, leave **Delete app data** unticked
to preserve the library. Export a backup before intentionally deleting app data.

`scripts/smoke-install.ps1` installs only into a checked `.local/installed/BookBuddy`
folder. It refuses unrelated current-user or machine-wide installations and asks
that other BookBuddy processes be closed. Baseline mode creates synthetic data;
Upgrade mode checks the same persistent dataset. Real release binaries use
`--acceptance-run=<UUID>` to select only a dedicated acceptance subdirectory;
the normal household database is never opened by these tests. Debug environment
overrides remain disabled in release builds.

For the installed release acceptance suite, set `BOOKBUDDY_SMOKE_EXECUTABLE` to
the installed executable and `BOOKBUDDY_SMOKE_INSTALLED=1`, then run
`scripts/offline-acceptance.ps1`. It exercises people, books, custom fields,
readings, read-only XLSX preview, transactional import/reimport, portable restore,
keyboard navigation and 100-row pagination. Each script chooses a fresh UUID.
WebView requests use an unreachable external proxy while local IPC remains usable.
Synthetic fixture emitters are opt-in Rust tests; no real household file is copied.

The keyboard test needs the test window to retain foreground keyboard focus. Do
not type in another application while it runs. It checks foreground ownership
before sending keys and stops if Windows keeps focus elsewhere. Use
`offline-acceptance.ps1 -SkipKeyboard` for unattended UI Automation runs, and run
the keyboard script separately when the desktop is available.

The public database tests cover migration upgrades, failure preservation,
independent personal information, sparse attributes, historical/actual readings,
repeat imports and complete backup/restore. The private-workbook tests are opt-in
and report only aggregate assertion outcomes.
