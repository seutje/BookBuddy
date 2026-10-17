# Windows release and acceptance

Build on Windows x64 with Node/npm, Rust MSVC, Visual Studio C++ tools and WebView2.
Use `npm ci`, the committed Cargo lockfile, and `npm run check` before packaging.
`scripts/windows-toolchain.ps1` imports the installed C++ compiler environment.

```powershell
. ./scripts/windows-toolchain.ps1
npm run tauri -- build --bundles nsis
```

The installer is generated at
`src-tauri/target/release/bundle/nsis/BookBuddy_0.1.1_x64-setup.exe`.
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
