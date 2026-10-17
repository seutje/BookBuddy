# Back up and recover BookBuddy

In Settings → Backups and restore, choose **Export backup** and a new filename.
The `.bookbuddy` file contains the complete library, including archived people,
personal ratings/notes, custom definitions and values, actual reading sessions,
historical borrowing, original workbook evidence and title reviews. No workbook
or network connection is needed to restore it. Save a copy on another drive or
device. Backups are unencrypted and include personal information.

Automatic backups run when BookBuddy opens, at most once every 24 hours. Settings
shows their folder, enabled state and retention count (default five; range 1–20).
They do not protect against losing the computer. Manual and pre-import/pre-restore
recovery files are preserved separately and are not automatically rotated.

To restore, choose **Choose backup to restore**. Inspect the dates and counts.
**Restore replaces the entire library; it does not merge.** Tick the replacement
confirmation and choose **Confirm replacement restore**. BookBuddy validates the
file again and creates a pre-restore recovery backup before the atomic replacement.
Close and reopen the app when it reports success. All previous person selections
and settings are replaced by the saved dataset as well.

If the library changed after preview, refresh and review again. If the archive
changed, choose it again. An invalid, incompatible or interrupted restore preserves
the current library; keep both original files for investigation. A newer backup
may require the compatible newer BookBuddy version. Existing export files are
never overwritten: choose another name.

Release data is in `%LOCALAPPDATA%\local.bookbuddy.desktop`; the exact folder is
shown in Settings. Do not copy only a live `bookbuddy.sqlite3`: recent changes may
still be in its WAL. Use Export backup while the app runs. Pre-import `.sqlite3`
snapshots are consistent standalone recovery files; preserve them and seek help
before replacing any database manually.

If BookBuddy cannot open its database, close every instance and retry. Do not delete
the data folder. To recover from a portable backup when it still cannot open:
close BookBuddy, copy the **entire** data folder to a safe location, then rename
the original folder to preserve it. Reopen to create a fresh library, create a
temporary local person, and restore the portable backup from Settings. Keep the
preserved folder until every restored record has been checked. Never move files
while another BookBuddy instance is running.

This MVP does not offer cover-file attachments. Backups with cover assets are
rejected explicitly so those files cannot be lost silently.
