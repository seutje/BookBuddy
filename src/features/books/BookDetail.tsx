import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  booksApi,
  choiceLabel,
  readingChoices,
  ratingChoices,
  listChoices,
} from "../../lib/books";
import type { BookDraft, PersonalDraft } from "../../lib/books";
import type { Person } from "../../lib/api";
import { errorMessage } from "../../lib/api";
import { BookForm } from "./BookForm";
import { PersonalForm } from "./PersonalForm";

type Action =
  | { type: "shared"; draft: BookDraft }
  | { type: "personal"; draft: PersonalDraft }
  | { type: "assign" }
  | { type: "read" };
export function BookDetail({
  id,
  current,
  onBack,
}: {
  id: string;
  current: Person | undefined;
  onBack: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["book", id],
    queryFn: () => booksApi.get(id),
  });
  const action = useMutation({
    mutationFn: (action: Action) => {
      if (action.type === "shared") return booksApi.update(id, action.draft);
      if (!current) return Promise.reject(new Error("Choose a person first."));
      if (action.type === "assign") return booksApi.ensure(current.id, id);
      if (action.type === "read") return booksApi.markRead(current.id, id);
      return booksApi.personal(current.id, id, action.draft);
    },
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["books"] }),
        client.invalidateQueries({ queryKey: ["book", id] }),
      ]);
      setEditing(false);
    },
  });
  const detail = query.data;
  const membership = detail?.memberships.find(
    (member) => member.personId === current?.id,
  );
  return (
    <section>
      <button className="quiet" disabled={action.isPending} onClick={onBack}>
        Back to Library
      </button>
      {query.error && <p role="alert">{errorMessage(query.error)}</p>}
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {!detail ? (
        <p role="status">Opening book…</p>
      ) : (
        <>
          <h1>{detail.book.title}</h1>
          <p>{detail.book.authors ?? "Author unknown"}</p>
          <h2>Shared book details</h2>
          <p className="muted">Changes here apply to the household catalog.</p>
          {editing ? (
            <BookForm
              initial={detail.book}
              bookId={id}
              pending={action.isPending}
              onSave={(draft) => action.mutateAsync({ type: "shared", draft })}
              onCancel={() => setEditing(false)}
            />
          ) : (
            <>
              <dl className="metadata">
                {[
                  ["Subtitle", detail.book.subtitle],
                  ["ISBN-13", detail.book.isbn13],
                  ["Publisher", detail.book.publisher],
                  ["Publication year", detail.book.publicationYear?.toString()],
                  ["Language", detail.book.language],
                  ["Series", detail.book.series],
                  ["Volume", detail.book.volume],
                  [
                    "Household ownership",
                    detail.book.owned === null
                      ? "Unknown"
                      : detail.book.owned
                        ? "Owned"
                        : "Not owned",
                  ],
                ].map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value || "Not set"}</dd>
                  </div>
                ))}
              </dl>
              {detail.book.description && (
                <p className="preserve-text">{detail.book.description}</p>
              )}
              {detail.book.notes && (
                <p className="preserve-text">
                  Shared notes: {detail.book.notes}
                </p>
              )}
              <button
                className="quiet"
                disabled={action.isPending}
                onClick={() => setEditing(true)}
              >
                Edit shared details
              </button>
            </>
          )}
          <section className="personal-panel">
            <h2>
              {current
                ? `${current.displayName}'s reading information`
                : "People and this book"}
            </h2>
            {current ? (
              membership ? (
                <>
                  <button
                    disabled={action.isPending}
                    onClick={() => {
                      void action.mutateAsync({ type: "read" }).catch(() => {});
                    }}
                  >
                    Mark read for {current.displayName}
                  </button>
                  <p className="muted">
                    Changes status only. No reading date is created.
                  </p>
                  <PersonalForm
                    key={JSON.stringify(membership)}
                    initial={membership}
                    name={current.displayName}
                    pending={action.isPending}
                    onSave={(draft) =>
                      action.mutateAsync({ type: "personal", draft })
                    }
                  />
                </>
              ) : (
                <>
                  <p>This book is not assigned to {current.displayName}.</p>
                  <button
                    disabled={action.isPending}
                    onClick={() => {
                      void action
                        .mutateAsync({ type: "assign" })
                        .catch(() => {});
                    }}
                  >
                    Add to {current.displayName}'s library
                  </button>
                </>
              )
            ) : (
              <p>
                Choose a person in the sidebar to add this book to their library
                or edit their reading information.
              </p>
            )}
            {detail.memberships.length > 0 && (
              <ul>
                {detail.memberships.map((member) => (
                  <li key={member.id}>
                    {member.personName}
                    {member.archivedAt ? " (archived)" : ""}:{" "}
                    {choiceLabel(member.readingStatus, readingChoices)} ·{" "}
                    {choiceLabel(member.rating, ratingChoices)} ·{" "}
                    {choiceLabel(member.listStatus, listChoices)}
                  </li>
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </section>
  );
}
