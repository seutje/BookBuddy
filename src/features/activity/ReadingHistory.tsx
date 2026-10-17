import { useState } from "react";
import { usePage } from "../../lib/pagination";
import { Pagination } from "../../components/Pagination";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { readingsApi } from "../../lib/readings";
import type {
  ReadingFilter,
  ReadingEvent,
  ReadingDraft,
} from "../../lib/readings";
import type { Person } from "../../lib/api";
import { errorMessage } from "../../lib/api";
import { ReadingForm } from "./ReadingForm";
import { QuickReading } from "./QuickReading";

export function ReadingHistory({
  filter,
  current,
  showRecent = false,
}: {
  filter: ReadingFilter;
  current: Person | undefined;
  showRecent?: boolean;
}) {
  const query = useQuery({
    queryKey: ["readings", filter],
    queryFn: () => readingsApi.list(filter),
  });
  const client = useQueryClient();
  const [editing, setEditing] = useState<ReadingEvent | null>(null);
  const [deleting, setDeleting] = useState<ReadingEvent | null>(null);
  const action = useMutation({
    mutationFn: async (
      work:
        | { type: "edit"; id: string; draft: ReadingDraft }
        | { type: "delete"; id: string },
    ) => {
      if (work.type === "edit") await readingsApi.edit(work.id, work.draft);
      else await readingsApi.delete(work.id);
    },
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["readings"] }),
        client.invalidateQueries({ queryKey: ["books"] }),
        client.invalidateQueries({ queryKey: ["book"] }),
      ]);
      setEditing(null);
      setDeleting(null);
    },
  });
  const page = usePage(query.data ?? [], JSON.stringify(filter));
  const recent: ReadingEvent[] = [];
  const seen = new Set<string>();
  for (const event of query.data ?? []) {
    if (!seen.has(event.bookId)) {
      recent.push(event);
      seen.add(event.bookId);
    }
    if (recent.length === 6) break;
  }
  return (
    <section aria-label="Reading history">
      {query.error && <p role="alert">{errorMessage(query.error)}</p>}
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {showRecent && current && recent.length > 0 && (
        <section aria-label="Recent books">
          <h2>Read again</h2>
          <div className="recent-books">
            {recent.map((event) => (
              <div key={event.bookId}>
                <strong>{event.title}</strong>
                <QuickReading
                  bookId={event.bookId}
                  title={event.title}
                  personId={current.id}
                  name={current.displayName}
                />
              </div>
            ))}
          </div>
        </section>
      )}
      {editing && (
        <section className="empty">
          <h3>Edit reading of {editing.title}</h3>
          <ReadingForm
            key={editing.id}
            initial={editing}
            current={current}
            pending={action.isPending}
            onSave={(draft) =>
              action.mutateAsync({ type: "edit", id: editing.id, draft })
            }
            onCancel={() => setEditing(null)}
          />
        </section>
      )}
      {deleting && (
        <div
          className="confirmation"
          role="group"
          aria-label="Delete reading confirmation"
        >
          <h3>Delete this reading?</h3>
          <p>
            {deleting.title} · {deleting.occurredOn} ·{" "}
            {deleting.participants
              .map((person) => person.displayName)
              .join(", ")}
          </p>
          <p>
            The event and its participant links will be removed. Personal
            status, notes and ratings are preserved.
          </p>
          <div className="actions">
            <button
              disabled={action.isPending}
              onClick={() => {
                void action
                  .mutateAsync({ type: "delete", id: deleting.id })
                  .catch(() => {});
              }}
            >
              Confirm delete reading
            </button>
            <button
              className="quiet"
              disabled={action.isPending}
              onClick={() => setDeleting(null)}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
      {query.isPending ? (
        <p role="status">Opening reading history…</p>
      ) : query.data?.length === 0 ? (
        <p>No reading events match this view.</p>
      ) : (
        <>
          <p role="status">{`${query.data?.length ?? 0} reading events`}</p>
          <ol className="reading-list">
            {page.items.map((event, index) => (
              <li key={event.id}>
                <div className="heading-row">
                  <div>
                    <strong>{event.title}</strong>
                    <p>
                      <time dateTime={event.occurredOn}>
                        {event.occurredOn}
                      </time>{" "}
                      ·{" "}
                      {event.participants
                        .map(
                          (person) =>
                            `${person.displayName}${person.archivedAt ? " (archived)" : ""}`,
                        )
                        .join(", ")}
                    </p>
                  </div>
                  <div className="actions">
                    <button
                      className="quiet"
                      disabled={action.isPending}
                      aria-label={`Edit reading ${page.offset + index + 1}`}
                      onClick={() => {
                        setEditing(event);
                        setDeleting(null);
                        action.reset();
                      }}
                    >
                      Edit
                    </button>
                    <button
                      className="quiet"
                      disabled={action.isPending}
                      aria-label={`Delete reading ${page.offset + index + 1}`}
                      onClick={() => {
                        setDeleting(event);
                        setEditing(null);
                        action.reset();
                      }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
                {event.note && <p className="preserve-text">{event.note}</p>}
              </li>
            ))}
          </ol>
          <Pagination label="readings" page={page} />
        </>
      )}
    </section>
  );
}
