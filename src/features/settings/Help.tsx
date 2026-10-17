export function Help() {
  return (
    <details className="personal-panel">
      <summary>Using BookBuddy</summary>
      <h2>Your first books</h2>
      <p>
        Choose a person in the sidebar, then Add book. Only the title is
        required. Shared details belong to the household; reading status,
        rating, list and personal notes belong to the person named on the panel.
        Choose Everyone to browse all shared books.
      </p>
      <h2>Reading together</h2>
      <p>
        Mark read changes status without inventing a date. Log read today
        records a dated session; repeat it for another reading. Activity lets
        you choose a past date and everyone who participated. Undo removes that
        session and preserves personal status and ratings.
      </p>
      <h2>Your own attributes</h2>
      <p>
        In Settings, Add attribute and choose Shared book or Each person's book.
        A new attribute is available on existing and future records. Unknown
        means no saved value; it differs from No, zero or empty text. Hiding an
        attribute preserves its values. Choice renames keep their identity.
      </p>
      <h2>Import a family workbook</h2>
      <p>
        Choose Excel workbook, inspect the sheets, map Carly and Mom, and review
        warnings and edition matches. Refresh after changing mappings. Review
        final import, then Confirm import. Historical borrowing stays separate
        from reading sessions. Blank ratings stay Unknown. Reimporting an
        unchanged file adds nothing; for a revised file choose its previous
        workbook source. Review unresolved titles in Title Verification.
      </p>
      <h2>Keep a recoverable copy</h2>
      <p>
        Export backup in Settings and save it on another drive. Restore replaces
        the whole dataset after preview and explicit confirmation. A pre-restore
        recovery backup is saved first. Close and reopen BookBuddy after
        restore.
      </p>
      <h2>Local privacy</h2>
      <p>
        Everything works offline. There are no accounts, analytics, cloud
        storage or automatic metadata downloads. People are organizational
        labels; everyone with access to this Windows user and data folder can
        see the household information. Backups contain personal data and are
        unencrypted.
      </p>
      <h2>Keyboard navigation</h2>
      <p>
        Use Tab and Shift+Tab to move between controls, Enter or Space to
        activate buttons and disclosure headings, and arrow keys in choice
        lists. Escape closes the person picker. The Skip to content link moves
        directly to the main panel. New-book and attribute editors focus their
        first field.
      </p>
    </details>
  );
}
