import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, errorMessage } from "../lib/api";
import { PersonForm } from "../components/PersonForm";
import { PersonPicker } from "../components/PersonPicker";
import { People } from "../features/people/People";
import { Library } from "../features/library/Library";

const pages = ["Library", "Activity", "People", "Settings"] as const;
type Page = (typeof pages)[number];
type Action =
  | { type: "create"; name: string }
  | { type: "rename"; id: string; name: string }
  | { type: "archive"; id: string }
  | { type: "select"; id: string | null };

export function App() {
  const [page, setPage] = useState<Page>("Library");
  const queryClient = useQueryClient();
  const startup = useQuery({ queryKey: ["startup"], queryFn: api.startup });
  const household = useQuery({
    queryKey: ["household"],
    queryFn: api.household,
    enabled: startup.data?.ready === true,
  });
  const action = useMutation({
    mutationFn: async (action: Action) => {
      switch (action.type) {
        case "create":
          return api.createPerson(action.name);
        case "rename":
          return api.renamePerson(action.id, action.name);
        case "archive":
          return api.archivePerson(action.id);
        case "select":
          return api.selectPerson(action.id);
      }
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["household"] });
    },
  });
  const people = household.data?.people ?? [];
  const active = people.filter((person) => !person.archivedAt);
  const current = active.find(
    (person) => person.id === household.data?.currentPersonId,
  );
  const ready = startup.data?.ready && household.data;
  const startupError = startup.error || startup.data?.error || household.error;

  return (
    <div className="shell">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <aside aria-label="BookBuddy">
        <strong className="brand">BookBuddy</strong>
        <p>Your household library</p>
        {ready && active.length > 0 && (
          <PersonPicker
            people={active}
            currentId={household.data.currentPersonId}
            pending={action.isPending}
            onSelect={(id) => action.mutateAsync({ type: "select", id })}
          />
        )}
        <nav aria-label="Main navigation">
          {pages.map((destination) => (
            <button
              key={destination}
              disabled={action.isPending}
              aria-current={page === destination ? "page" : undefined}
              onClick={() => {
                setPage(destination);
                action.reset();
              }}
            >
              {destination}
            </button>
          ))}
        </nav>
        <p className="sidebar-note">
          Stored on this computer.
          <br />
          No account needed.
        </p>
      </aside>
      <main id="main" tabIndex={-1}>
        {startupError ? (
          <div role="alert">
            <h1>Your library could not be opened</h1>
            <p>{errorMessage(startupError)}</p>
            <p>
              Close and reopen BookBuddy to retry. Keep the existing data folder
              for recovery.
            </p>
            {startup.data?.dataDirectory && (
              <label>
                Data folder
                <input readOnly value={startup.data.dataDirectory} />
              </label>
            )}
          </div>
        ) : !ready ? (
          <p role="status">Opening your local library…</p>
        ) : (
          <>
            {action.error && (
              <p role="alert" className="error">
                {errorMessage(action.error)}
              </p>
            )}
            {active.length === 0 ? (
              <section>
                <h1>Welcome to BookBuddy</h1>
                <p>
                  A place for your books and the people who read them. Start
                  with one person's name.
                </p>
                <PersonForm
                  pending={action.isPending}
                  submitLabel="Create person"
                  onSave={(name) =>
                    action.mutateAsync({ type: "create", name })
                  }
                />
                {people.length > 0 && (
                  <p>
                    Previously archived people and their history are preserved.
                  </p>
                )}
              </section>
            ) : (
              <>
                <p className="context" aria-live="polite">
                  {current
                    ? `Viewing ${current.displayName}’s library`
                    : "Viewing everyone in the household"}
                </p>
                {page === "Library" && <Library current={current} />}
                {page === "Activity" && (
                  <section>
                    <h1>Activity</h1>
                    <div className="empty">
                      <h2>No readings logged</h2>
                      <p>
                        Reading sessions will appear here, including books you
                        read together.
                      </p>
                    </div>
                  </section>
                )}
                {page === "People" && (
                  <People
                    people={people}
                    pending={action.isPending}
                    create={(name) =>
                      action.mutateAsync({ type: "create", name })
                    }
                    rename={(id, name) =>
                      action.mutateAsync({ type: "rename", id, name })
                    }
                    archive={(id) =>
                      action.mutateAsync({ type: "archive", id })
                    }
                  />
                )}
                {page === "Settings" && (
                  <section>
                    <h1>Settings</h1>
                    <h2>Your local data</h2>
                    <p>
                      Books and reading information stay on this computer.
                      Household people are labels, with no separate accounts.
                    </p>
                    <label>
                      Data folder
                      <input
                        readOnly
                        value={startup.data?.dataDirectory ?? "Unavailable"}
                      />
                    </label>
                    <p className="muted">
                      Keep this folder when troubleshooting. Do not copy a
                      database while BookBuddy is running.
                    </p>
                  </section>
                )}
              </>
            )}
          </>
        )}
      </main>
    </div>
  );
}
