import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  booksApi,
  readingChoices,
  ratingChoices,
  listChoices,
  choiceLabel,
} from "../../lib/books";
import type { BookDraft, BookFilter, Filter } from "../../lib/books";
import type { Person } from "../../lib/api";
import { errorMessage } from "../../lib/api";
import { BookForm } from "../books/BookForm";
import { BookDetail } from "../books/BookDetail";

function FilterSelect<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: Filter<T>;
  options: readonly { value: T; label: string }[];
  onChange: (value: Filter<T>) => void;
}) {
  return (
    <label>
      {label}
      <select
        value={value.mode === "equals" ? value.value : value.mode}
        onChange={(event) => {
          const option = options.find(
            (option) => option.value === event.target.value,
          );
          onChange(
            option
              ? { mode: "equals", value: option.value }
              : event.target.value === "unknown"
                ? { mode: "unknown" }
                : { mode: "any" },
          );
        }}
      >
        <option value="any">Any</option>
        <option value="unknown">Unknown / not set</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
export function Library({ current }: { current: Person | undefined }) {
  const [filter, setFilter] = useState<Omit<BookFilter, "personId">>({
    search: "",
    readingStatus: { mode: "any" },
    rating: { mode: "any" },
    listStatus: { mode: "any" },
    owned: { mode: "any" },
    sort: "recently_added",
  });
  const [adding, setAdding] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [assign, setAssign] = useState(true);
  const client = useQueryClient();
  const request: BookFilter = { ...filter, personId: current?.id ?? null };
  const query = useQuery({
    queryKey: ["books", request],
    queryFn: () => booksApi.list(request),
  });
  const action = useMutation({
    mutationFn: async (
      action:
        { type: "create"; draft: BookDraft } | { type: "read"; id: string },
    ) => {
      if (action.type === "create") {
        const id = await booksApi.create(
          action.draft,
          assign ? (current?.id ?? null) : null,
        );
        setAdding(false);
        setSelected(id);
      } else if (current) await booksApi.markRead(current.id, action.id);
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ["books"] });
    },
  });
  if (selected)
    return (
      <BookDetail
        key={current?.id ?? "everyone"}
        id={selected}
        current={current}
        onBack={() => setSelected(null)}
      />
    );
  return (
    <section>
      <div className="heading-row">
        <h1>Library</h1>
        <button onClick={() => setAdding(true)} disabled={action.isPending}>
          Add book
        </button>
      </div>
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {adding ? (
        <section className="empty" aria-labelledby="add-heading">
          <h2 id="add-heading">Add book</h2>
          <p>Title is all you need. Shared details can be added later.</p>
          {current && (
            <label className="checkbox">
              <input
                type="checkbox"
                checked={assign}
                onChange={(event) => setAssign(event.target.checked)}
                disabled={action.isPending}
              />
              Add to {current.displayName}'s library
            </label>
          )}
          {!current && (
            <p>
              This adds a shared catalog entry. Assign it to a person from the
              book's detail view.
            </p>
          )}
          <BookForm
            pending={action.isPending}
            onSave={(draft) => action.mutateAsync({ type: "create", draft })}
            onCancel={() => setAdding(false)}
          />
        </section>
      ) : (
        <>
          <label>
            Search books
            <input
              type="search"
              value={filter.search}
              onChange={(event) =>
                setFilter({ ...filter, search: event.target.value })
              }
              placeholder="Title, author, ISBN or text"
            />
          </label>
          <details className="filters">
            <summary>Filters and sort</summary>
            <div className="form-grid">
              <FilterSelect
                label="Reading status filter"
                value={filter.readingStatus}
                options={readingChoices}
                onChange={(readingStatus) =>
                  setFilter({ ...filter, readingStatus })
                }
              />
              <FilterSelect
                label="Rating filter"
                value={filter.rating}
                options={ratingChoices}
                onChange={(rating) => setFilter({ ...filter, rating })}
              />
              <FilterSelect
                label="Personal list filter"
                value={filter.listStatus}
                options={listChoices}
                onChange={(listStatus) => setFilter({ ...filter, listStatus })}
              />
              <label>
                Ownership filter
                <select
                  value={
                    filter.owned.mode === "equals"
                      ? filter.owned.value
                        ? "yes"
                        : "no"
                      : filter.owned.mode
                  }
                  onChange={(event) => {
                    const value = event.target.value;
                    setFilter({
                      ...filter,
                      owned:
                        value === "yes" || value === "no"
                          ? { mode: "equals", value: value === "yes" }
                          : value === "unknown"
                            ? { mode: "unknown" }
                            : { mode: "any" },
                    });
                  }}
                >
                  <option value="any">Any</option>
                  <option value="unknown">Unknown</option>
                  <option value="yes">Owned</option>
                  <option value="no">Not owned</option>
                </select>
              </label>
              <label>
                Sort by
                <select
                  value={filter.sort}
                  onChange={(event) => {
                    const value = event.target.value;
                    if (
                      value === "recently_added" ||
                      value === "title" ||
                      value === "author" ||
                      value === "rating"
                    )
                      setFilter({ ...filter, sort: value });
                  }}
                >
                  <option value="recently_added">Recently added</option>
                  <option value="title">Title</option>
                  <option value="author">Author</option>
                  <option value="rating">Rating</option>
                </select>
              </label>
            </div>
          </details>
          {query.error ? (
            <p role="alert">{errorMessage(query.error)}</p>
          ) : query.isPending ? (
            <p role="status">Finding books…</p>
          ) : query.data.length === 0 ? (
            <div className="empty">
              <h2>No books here yet</h2>
              <p>
                Add a book or change your search, filters or current person.
                Choose Everyone to browse the shared catalog.
              </p>
            </div>
          ) : (
            <>
              <p className="muted" role="status">
                {`${query.data.length} ${query.data.length === 1 ? "book" : "books"}`}
              </p>
              <ul className="book-list">
                {query.data.map(({ book, memberships }) => (
                  <li key={book.id}>
                    <div>
                      <button
                        className="book-title"
                        onClick={() => setSelected(book.id)}
                      >
                        {book.title}
                      </button>
                      <p>{book.authors ?? "Author unknown"}</p>
                      <p className="muted">
                        Household:{" "}
                        {book.owned === null
                          ? "ownership unknown"
                          : book.owned
                            ? "owned"
                            : "not owned"}
                      </p>
                      <ul className="membership-list">
                        {memberships
                          .filter(
                            (member) =>
                              !current || member.personId === current.id,
                          )
                          .map((member) => (
                            <li key={member.id}>
                              {member.personName}
                              {member.archivedAt ? " (archived)" : ""}:{" "}
                              {choiceLabel(
                                member.readingStatus,
                                readingChoices,
                              )}{" "}
                              · {choiceLabel(member.rating, ratingChoices)}
                            </li>
                          ))}
                      </ul>
                    </div>
                    {current && (
                      <button
                        className="quiet"
                        disabled={action.isPending}
                        aria-label={`Mark ${book.title} read for ${current.displayName}`}
                        onClick={() => {
                          void action
                            .mutateAsync({ type: "read", id: book.id })
                            .catch(() => {});
                        }}
                      >
                        Mark read
                      </button>
                    )}
                  </li>
                ))}
              </ul>
            </>
          )}
        </>
      )}
    </section>
  );
}
