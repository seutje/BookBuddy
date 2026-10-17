import { useState } from "react";
import type { FormEvent } from "react";
import { useQuery } from "@tanstack/react-query";
import { booksApi, emptyBook } from "../../lib/books";
import type { BookDraft } from "../../lib/books";
import { errorMessage } from "../../lib/api";

interface Props {
  initial?: BookDraft;
  bookId?: string;
  pending: boolean;
  onSave: (draft: BookDraft) => Promise<unknown>;
  onCancel: () => void;
}
export function BookForm({
  initial = emptyBook,
  bookId,
  pending,
  onSave,
  onCancel,
}: Props) {
  const [draft, setDraft] = useState<BookDraft>(() => ({
    title: initial.title,
    subtitle: initial.subtitle,
    authors: initial.authors,
    isbn13: initial.isbn13,
    publisher: initial.publisher,
    publicationYear: initial.publicationYear,
    language: initial.language,
    series: initial.series,
    volume: initial.volume,
    description: initial.description,
    notes: initial.notes,
    owned: initial.owned,
  }));
  const hints = useQuery({
    queryKey: [
      "duplicate-hints",
      draft.title,
      draft.authors,
      draft.isbn13,
      bookId,
    ],
    queryFn: () => booksApi.hints(draft, bookId ?? null),
    enabled: draft.title.trim().length > 0,
  });
  function text(
    key:
      | "subtitle"
      | "authors"
      | "isbn13"
      | "publisher"
      | "language"
      | "series"
      | "volume",
    label: string,
  ) {
    return (
      <label key={key}>
        {label}
        <input
          value={draft[key] ?? ""}
          maxLength={key === "isbn13" ? 32 : 5000}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, [key]: event.target.value || null })
          }
        />
      </label>
    );
  }
  async function submit(event: FormEvent) {
    event.preventDefault();
    try {
      await onSave(draft);
    } catch {
      /* Parent displays the backend error. */
    }
  }
  return (
    <form
      className="book-form"
      onSubmit={(event) => {
        void submit(event);
      }}
    >
      <label>
        Title
        <input
          required
          maxLength={1000}
          autoFocus
          value={draft.title}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, title: event.target.value })
          }
        />
      </label>
      {text("authors", "Author(s)")}
      {hints.error && (
        <p role="alert">Duplicate check: {errorMessage(hints.error)}</p>
      )}
      {hints.data && hints.data.length > 0 && (
        <div className="notice">
          <strong>Possible existing books</strong>
          <ul>
            {hints.data.map((book) => (
              <li key={book.id}>
                {book.title}
                {book.authors ? ` — ${book.authors}` : ""}
              </li>
            ))}
          </ul>
          <p>
            Check the catalog before saving. Saving creates a separate entry;
            books are never merged automatically.
          </p>
        </div>
      )}
      <details open={bookId ? true : undefined}>
        <summary>More shared details</summary>
        <div className="form-grid">
          {text("subtitle", "Subtitle")}
          {text("isbn13", "ISBN-13")}
          {text("publisher", "Publisher")}
          <label>
            Publication year
            <input
              type="number"
              min={1}
              max={3000}
              step={1}
              value={draft.publicationYear ?? ""}
              disabled={pending}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  publicationYear: event.target.value
                    ? Number(event.target.value)
                    : null,
                })
              }
            />
          </label>
          {text("language", "Language")}
          {text("series", "Series")}
          {text("volume", "Volume")}
          <label>
            Household owns this book
            <select
              value={
                draft.owned === null ? "unknown" : draft.owned ? "yes" : "no"
              }
              disabled={pending}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  owned:
                    event.target.value === "unknown"
                      ? null
                      : event.target.value === "yes",
                })
              }
            >
              <option value="unknown">Unknown</option>
              <option value="yes">Yes</option>
              <option value="no">No</option>
            </select>
          </label>
        </div>
        <label>
          Description
          <textarea
            value={draft.description ?? ""}
            maxLength={100000}
            disabled={pending}
            onChange={(event) =>
              setDraft({ ...draft, description: event.target.value })
            }
          />
        </label>
        <label>
          Shared notes
          <textarea
            value={draft.notes ?? ""}
            maxLength={100000}
            disabled={pending}
            onChange={(event) =>
              setDraft({ ...draft, notes: event.target.value })
            }
          />
        </label>
      </details>
      <div className="actions">
        <button disabled={pending || !draft.title.trim()}>
          {pending ? "Saving…" : bookId ? "Save shared details" : "Save book"}
        </button>
        <button
          type="button"
          className="quiet"
          disabled={pending}
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}
