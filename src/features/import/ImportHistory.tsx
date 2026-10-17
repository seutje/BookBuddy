import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { errorMessage } from "../../lib/api";
import { allBooksFilter, booksApi } from "../../lib/books";
import { importApi } from "../../lib/import";
import type { TitleTask } from "../../lib/import";

function TitleReview({ task }: { task: TitleTask }) {
  const client = useQueryClient();
  const books = useQuery({
    queryKey: ["books", allBooksFilter],
    queryFn: () => booksApi.list(allBooksFilter),
  });
  const [bookId, setBookId] = useState(task.bookId ?? "");
  const [title, setTitle] = useState(
    task.proposedConfirmedTitle ?? task.currentTitle ?? task.workingTitle,
  );
  const [correct, setCorrect] = useState(false);
  const current = books.data?.find((row) => row.book.id === bookId)?.book;
  const action = useMutation({
    mutationFn: () =>
      importApi.resolve(task.id, {
        bookId: bookId || null,
        confirmedTitle: correct ? title : null,
        expectedCurrentTitle: current?.title ?? null,
        expectedState: task.state,
      }),
    onSuccess: () => client.invalidateQueries(),
  });
  return (
    <details className="import-row">
      <summary>
        {task.workingTitle} · {task.state}
      </summary>
      <p>PDF wording: {task.rawPdfTitle ?? "Not supplied"}</p>
      <p>Reason: {task.reason ?? "Source requested verification"}</p>
      <p>Current shared title: {task.currentTitle ?? "Unlinked"}</p>
      <label>
        Shared book to review
        <select
          disabled={!!task.bookId || action.isPending}
          value={bookId}
          onChange={(event) => setBookId(event.target.value)}
        >
          <option value="">Keep as an unlinked source task</option>
          {books.data?.map((row) => (
            <option key={row.book.id} value={row.book.id}>
              {row.book.title}
              {row.book.authors ? ` — ${row.book.authors}` : ""}
            </option>
          ))}
        </select>
      </label>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={correct}
          disabled={!bookId || action.isPending}
          onChange={(event) => setCorrect(event.target.checked)}
        />
        Apply a reviewed shared-title correction
      </label>
      {correct && (
        <label>
          Reviewed full title
          <input
            value={title}
            maxLength={1000}
            onChange={(event) => setTitle(event.target.value)}
          />
        </label>
      )}
      <p>
        {correct
          ? `The shared title “${current?.title ?? ""}” will become “${title}” for everyone. Original source wording stays in import history.`
          : "Confirm this review without changing the shared title. Original source wording stays in import history."}
      </p>
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      <button
        disabled={
          action.isPending ||
          (!!bookId && !current) ||
          (correct && !title.trim())
        }
        onClick={() => action.mutate()}
      >
        Confirm title review
      </button>
    </details>
  );
}

export function HistoricalBorrowing({
  bookId = null,
}: {
  bookId?: string | null;
}) {
  const client = useQueryClient();
  const loans = useQuery({
    queryKey: ["historicalLoans", bookId],
    queryFn: () => importApi.loans(bookId),
  });
  const baseline = useQuery({
    queryKey: ["borrowBaseline", bookId],
    queryFn: () => importApi.baseline(bookId!),
    enabled: !!bookId,
  });
  const books = useQuery({
    queryKey: ["books", allBooksFilter],
    queryFn: () => booksApi.list(allBooksFilter),
    enabled: !bookId,
  });
  const [links, setLinks] = useState<Record<string, string>>({});
  const [reviewId, setReviewId] = useState<string | null>(null);
  const action = useMutation({
    mutationFn: (id: string) => {
      const loan = loans.data?.find((loan) => loan.id === id);
      if (!loan || !links[id]) throw new Error("Choose a shared book first.");
      return importApi.linkLoan(id, links[id], loan.bookId);
    },
    onSuccess: async () => {
      setReviewId(null);
      await client.invalidateQueries();
    },
  });
  return (
    <section className="personal-panel">
      <h2>Historical borrowing</h2>
      <p>
        Imported library loans are separate from reading sessions. Dates are
        unverified source dates and may be return-date proxies.
      </p>
      {baseline.data && (
        <p>
          Source borrow-count baseline:{" "}
          {baseline.data.importedCountBaseline ?? "Unknown"}. Linked historical
          rows: {baseline.data.linkedHistoryCount}. Differences are retained; no
          missing loans are invented.
        </p>
      )}
      {loans.error && (
        <p role="alert" className="error">
          {errorMessage(loans.error)}
        </p>
      )}
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {loans.isPending ? (
        <p role="status">Loading historical borrowing…</p>
      ) : loans.data?.length === 0 ? (
        <p>No imported borrowing records.</p>
      ) : (
        loans.data?.map((loan) => (
          <details className="import-row" key={loan.id}>
            <summary>
              {loan.sourceTitle} · {loan.datePrecision} precision ·{" "}
              {loan.currentTitle ? "Linked" : "Unlinked"}
            </summary>
            <p>
              Person: {loan.personName}. Source author:{" "}
              {loan.sourceAuthor ?? "Unknown"}. Format:{" "}
              {loan.sourceFormat ?? "Unknown"}.
            </p>
            <p>
              Date interpretation: {loan.dateInterpretation}. PDF label:{" "}
              {loan.sourcePdf ?? "Not supplied"}.
            </p>
            <details>
              <summary>Original date value and precision</summary>
              <pre className="preserve-text">{loan.sourceDateJson}</pre>
            </details>
            <p>Shared book: {loan.currentTitle ?? "Unlinked for review"}</p>
            {!bookId && (
              <>
                <label>
                  Reviewed book link
                  <select
                    value={links[loan.id] ?? ""}
                    onChange={(event) => {
                      setLinks({ ...links, [loan.id]: event.target.value });
                      setReviewId(null);
                    }}
                  >
                    <option value="">Choose an exact book or edition</option>
                    {books.data?.map((row) => (
                      <option key={row.book.id} value={row.book.id}>
                        {row.book.title}
                        {row.book.authors ? ` — ${row.book.authors}` : ""}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  disabled={!links[loan.id] || action.isPending}
                  onClick={() => setReviewId(loan.id)}
                >
                  Review borrowing link
                </button>
                {reviewId === loan.id && (
                  <div className="notice">
                    <p>
                      Link this original loan row to “
                      {
                        books.data?.find(
                          (row) => row.book.id === links[loan.id],
                        )?.book.title
                      }
                      ”. This replaces its current link and preserves all
                      original source cells.
                    </p>
                    <button
                      disabled={action.isPending}
                      onClick={() => action.mutate(loan.id)}
                    >
                      Confirm borrowing link
                    </button>
                  </div>
                )}
              </>
            )}
          </details>
        ))
      )}
    </section>
  );
}

function SavedRows({ batchId }: { batchId: string }) {
  const rows = useQuery({
    queryKey: ["importRows", batchId],
    queryFn: () => importApi.rows(batchId),
  });
  return (
    <>
      {rows.isPending && <p role="status">Loading original rows…</p>}
      {rows.error && <p role="alert">{errorMessage(rows.error)}</p>}
      {rows.data?.map((row) => (
        <details key={row.id}>
          <summary>
            {row.sheetName} row {row.sheetRowNumber} ·{" "}
            {row.bookId
              ? "Book linked"
              : row.loanId
                ? "Loan linked"
                : row.verificationId
                  ? "Title task linked"
                  : "Source retained"}
          </summary>
          <pre className="preserve-text">{row.rawJson}</pre>
          <p>Warnings</p>
          <pre className="preserve-text">{row.warningJson}</pre>
        </details>
      ))}
    </>
  );
}
export function ImportHistory() {
  const [includeResolved, setIncludeResolved] = useState(false);
  const [batchId, setBatchId] = useState<string | null>(null);
  const batches = useQuery({
    queryKey: ["importBatches"],
    queryFn: importApi.batches,
  });
  const tasks = useQuery({
    queryKey: ["titleTasks", includeResolved],
    queryFn: () => importApi.tasks(includeResolved),
  });
  return (
    <>
      <section className="personal-panel">
        <h2>Title Verification</h2>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={includeResolved}
            onChange={(event) => setIncludeResolved(event.target.checked)}
          />
          Include resolved title reviews
        </label>
        {tasks.error && <p role="alert">{errorMessage(tasks.error)}</p>}
        {tasks.isPending && <p role="status">Loading title tasks…</p>}
        {tasks.data?.length === 0 && <p>No titles need verification.</p>}
        {tasks.data?.map((task) => (
          <TitleReview
            key={`${task.id}:${task.state}:${task.currentTitle}`}
            task={task}
          />
        ))}
      </section>
      <section className="personal-panel">
        <h2>Import History</h2>
        {batches.error && <p role="alert">{errorMessage(batches.error)}</p>}
        {batches.isPending && <p role="status">Loading import history…</p>}
        {batches.data?.length === 0 && <p>No workbooks imported yet.</p>}
        {batches.data?.map((batch) => (
          <details key={batch.id}>
            <summary>
              {batch.sourceFilename} · {batch.importedAt}
            </summary>
            <p>
              File SHA-256: {batch.sourceHash}. Mapping version{" "}
              {batch.mappingVersion}.
            </p>
            <details>
              <summary>Saved import summary</summary>
              <pre className="preserve-text">{batch.summaryJson}</pre>
            </details>
            <button onClick={() => setBatchId(batch.id)}>
              Inspect original source rows
            </button>
            {batchId === batch.id && <SavedRows batchId={batch.id} />}
          </details>
        ))}
      </section>
      <HistoricalBorrowing />
    </>
  );
}
