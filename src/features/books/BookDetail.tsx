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
import { CustomFields } from "../custom-fields/CustomFields";
import { ReadingHistory } from "../activity/ReadingHistory";
import { QuickReading } from "../activity/QuickReading";
import { ReadingForm } from "../activity/ReadingForm";
import { readingsApi } from "../../lib/readings";
import type { ReadingDraft } from "../../lib/readings";

type Action =
  | { type: "shared"; draft: BookDraft }
  | { type: "personal"; draft: PersonalDraft }
  | { type: "assign" }
  | { type: "read" }
  | { type: "log"; draft: ReadingDraft };
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
  const [logging, setLogging] = useState(false);
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["book", id],
    queryFn: () => booksApi.get(id),
  });
  const action = useMutation({
    mutationFn: async (action: Action) => {
      if (action.type === "log") {
        await readingsApi.log(action.draft);
        return;
      }
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
        client.invalidateQueries({ queryKey: ["readings"] }),
      ]);
      setEditing(false);
      setLogging(false);
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
          <p className="muted">{`Household: ${detail.book.readCount} logged readings · Last reading: ${detail.book.lastReadOn ?? "Unknown"}`}</p>
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
          <CustomFields target={{ scope: "book", bookId: id }} />
          <section className="personal-panel" aria-label="Log a reading">
            <h2>Log a reading</h2>
            {current && (
              <QuickReading
                bookId={id}
                title={detail.book.title}
                personId={current.id}
                name={current.displayName}
              />
            )}
            <button
              className="quiet"
              disabled={action.isPending}
              onClick={() => setLogging(true)}
            >
              Choose date and participants
            </button>
            {logging && (
              <ReadingForm
                bookId={id}
                current={current}
                pending={action.isPending}
                onSave={(draft) => action.mutateAsync({ type: "log", draft })}
                onCancel={() => setLogging(false)}
              />
            )}
          </section>
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
                  <p>{`${current.displayName}: ${membership.readCount} logged readings · Last reading: ${membership.lastReadOn ?? "Unknown"}`}</p>
                  <PersonalForm
                    key={membership.id}
                    initial={membership}
                    name={current.displayName}
                    pending={action.isPending}
                    onSave={(draft) =>
                      action.mutateAsync({ type: "personal", draft })
                    }
                  />
                  <CustomFields
                    target={{
                      scope: "person_book",
                      bookId: id,
                      personId: current.id,
                    }}
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
          <section className="personal-panel">
            <h2>Reading history</h2>
            <ReadingHistory
              current={current}
              filter={{
                personId: current?.id ?? null,
                bookId: id,
                from: null,
                to: null,
              }}
            />
          </section>
        </>
      )}
    </section>
  );
}
