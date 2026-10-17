# Import the family library workbook

Keep the original workbook and a recovery copy. BookBuddy reads local `.xlsx`
files and recognizes Book Catalogue, Library History, Check Titles, Dashboard
and How to Use by name. It maps columns by headers, including shifted/reordered
columns. Book Catalogue and Title are essential. Missing History/Check Titles
requires explicit acknowledgement; reference-sheet omissions produce warnings.

Choose **Import Excel library** on the welcome screen or in Settings:

1. **Choose Excel workbook** with the Windows picker. Inspection does not write
   books, people or attributes.
2. Inspect detected sheets/counts, column mappings, original cells, warnings and
   borrow-count reconciliation. Unsupported values stay in source evidence.
3. Map Carly and Mom to distinct new or existing active people. Carly's reading
   status and rating remain separate from Mom's categorical rating. Blank ratings
   stay Unknown; a Mom rating never sets her reading status. A blank Mom rating
   creates no personal membership.
4. Review catalog collisions and editions. Explicitly choose an existing book,
   a separate new book, or skip while retaining provenance. Approximate title
   similarity never merges records. Unmatched loans/title tasks can remain unlinked.
5. Review shared/personal attribute mappings and ambiguous slash dates. Partial
   dates keep their precision. Refresh the preview after changing mappings or
   acknowledgements.
6. **Review final import**, then **Confirm import**. A recovery snapshot is saved
   before one transaction commits all records. File and library changes invalidate
   stale review; failures roll back. The resulting summary and Import History show
   what happened and retain original row evidence.

Library History contains historical borrowing, not actual readings. Original date
types and source PDF labels remain visible. Approximate dates may be return-date
proxies. Identical-looking rows remain separate loans if they were separate source
occurrences. Source borrow counts are baselines; missing transactions are never
fabricated to make counts agree.

Check Titles becomes Title Verification. Original working/PDF wording stays saved.
Applying a confirmed source title requires an explicit choice. The post-import
queue also supports reviewing an exact book link and confirming a correction for
the shared catalog. Needs title verification is a Library filter. Resolve uncertain
links and editions manually; no online guesses are made.

An unchanged file imported again adds nothing. For revised workbooks select the
previous workbook source under **Workbook source continuity**. Its saved row,
field/choice identities and reviewed links are preserved even after manual renames.
Review new or ambiguous rows. Existing populated values take precedence, including
No, zero and empty saved text; imported blanks never erase them. Explicit confirmed
title corrections are the only overwrite option. Resolved tasks and loan links
stay as reviewed. Changed historical evidence is a distinct source row and needs
review rather than silently rewriting a past loan.

The importer accepts files up to 32 MiB compressed and 128 MiB expanded, with
limits on sheets, cells and coordinates. It uses cached formula results and never
executes formulas, macros, HTML or workbook text. Dashboard and How to Use are
reference evidence and do not create books.
