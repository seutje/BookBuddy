import { useId, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { readingsApi } from "../../lib/readings";
import type { ReadingDraft, ReadingEvent } from "../../lib/readings";
import { booksApi, allBooksFilter } from "../../lib/books";
import { api, errorMessage } from "../../lib/api";
import type { Person } from "../../lib/api";

interface Props {
  bookId?: string;
  current: Person | undefined;
  initial?: ReadingEvent;
  pending: boolean;
  onSave: (draft: ReadingDraft) => Promise<unknown>;
  onCancel: () => void;
}
function FormBody({
  today,
  people,
  books,
  ...props
}: Props & {
  today: string;
  people: Person[];
  books: { id: string; title: string }[];
}) {
  const [draft, setDraft] = useState<ReadingDraft>(() => ({
    bookId: props.initial?.bookId ?? props.bookId ?? "",
    occurredOn: props.initial?.occurredOn ?? today,
    participantIds:
      props.initial?.participants.map((participant) => participant.personId) ??
      (props.current ? [props.current.id] : []),
    note: props.initial?.note ?? null,
    setRead: !props.initial,
  }));
  const allowedPeople = people.filter(
    (person) =>
      !person.archivedAt ||
      props.initial?.participants.some(
        (participant) => participant.personId === person.id,
      ),
  );
  return (
    <form
      className="book-form"
      onSubmit={(event) => {
        event.preventDefault();
        void props.onSave(draft).catch(() => {});
      }}
    >
      <label>
        Reading book
        <select
          autoFocus={!props.initial && !props.bookId}
          required
          value={draft.bookId}
          disabled={props.pending || !!props.initial || !!props.bookId}
          onChange={(event) =>
            setDraft({ ...draft, bookId: event.target.value })
          }
        >
          <option value="">Choose a book</option>
          {books.map((book) => (
            <option key={book.id} value={book.id}>
              {book.title}
            </option>
          ))}
        </select>
      </label>
      <label>
        Reading date
        <input
          type="date"
          autoFocus={!!props.initial || !!props.bookId}
          required
          max={today}
          value={draft.occurredOn}
          disabled={props.pending}
          onChange={(event) =>
            setDraft({ ...draft, occurredOn: event.target.value })
          }
        />
      </label>
      <p className="muted">
        Local calendar date. A shared session is one event with everyone who
        took part.
      </p>
      <fieldset>
        <legend>Participants</legend>
        {allowedPeople.map((person) => (
          <label className="checkbox" key={person.id}>
            <input
              type="checkbox"
              checked={draft.participantIds.includes(person.id)}
              disabled={props.pending}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  participantIds: event.target.checked
                    ? [...draft.participantIds, person.id]
                    : draft.participantIds.filter((id) => id !== person.id),
                })
              }
            />
            {person.displayName}
            {person.archivedAt ? " (archived participant)" : ""}
          </label>
        ))}
      </fieldset>
      <label>
        Reading note
        <textarea
          value={draft.note ?? ""}
          maxLength={100000}
          disabled={props.pending}
          onChange={(event) => setDraft({ ...draft, note: event.target.value })}
        />
      </label>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={draft.setRead}
          disabled={props.pending}
          onChange={(event) =>
            setDraft({ ...draft, setRead: event.target.checked })
          }
        />
        Set active participants' status to Read
      </label>
      <p className="muted">Ratings, lists and personal notes are preserved.</p>
      <div className="actions">
        <button
          disabled={
            props.pending || !draft.bookId || !draft.participantIds.length
          }
        >
          {props.pending
            ? "Saving…"
            : props.initial
              ? "Save reading changes"
              : "Save reading"}
        </button>
        <button
          type="button"
          className="quiet"
          disabled={props.pending}
          onClick={props.onCancel}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}
export function ReadingForm(props: Props) {
  const instance = useId();
  const today = useQuery({
    queryKey: ["local-today", instance],
    queryFn: readingsApi.today,
  });
  const people = useQuery({ queryKey: ["household"], queryFn: api.household });
  const books = useQuery({
    queryKey: ["books", allBooksFilter],
    queryFn: () => booksApi.list(allBooksFilter),
  });
  const error = today.error || people.error || books.error;
  if (error) return <p role="alert">{errorMessage(error)}</p>;
  if (!today.data || !people.data || !books.data)
    return <p role="status">Preparing reading…</p>;
  return (
    <FormBody
      {...props}
      today={today.data}
      people={people.data.people}
      books={books.data.map((row) => row.book)}
    />
  );
}
