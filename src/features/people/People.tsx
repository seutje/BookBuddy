import { useState } from "react";
import { PersonForm } from "../../components/PersonForm";
import type { Person } from "../../lib/api";

interface Props {
  people: Person[];
  pending: boolean;
  create: (name: string) => Promise<void>;
  rename: (id: string, name: string) => Promise<void>;
  archive: (id: string) => Promise<void>;
}

export function People({ people, pending, create, rename, archive }: Props) {
  const [editing, setEditing] = useState<string | null>(null);
  const [archiving, setArchiving] = useState<string | null>(null);
  return (
    <section aria-labelledby="people-heading">
      <h1 id="people-heading">People</h1>
      <p>
        Each person has their own reading information. Everyone using this
        computer can see the household library.
      </p>
      <ul className="people-list">
        {people.map((person) => (
          <li key={person.id}>
            <div className="person-row">
              <strong>{person.displayName}</strong>
              {person.archivedAt ? (
                <span className="muted">Archived · history preserved</span>
              ) : (
                <div className="actions">
                  <button
                    className="quiet"
                    disabled={pending}
                    onClick={() => {
                      setEditing(person.id);
                      setArchiving(null);
                    }}
                    aria-label={`Rename ${person.displayName}`}
                  >
                    Rename
                  </button>
                  <button
                    className="quiet"
                    disabled={pending}
                    onClick={() => {
                      setArchiving(person.id);
                      setEditing(null);
                    }}
                    aria-label={`Archive ${person.displayName}`}
                  >
                    Archive
                  </button>
                </div>
              )}
            </div>
            {editing === person.id && (
              <PersonForm
                key={person.id}
                initialName={person.displayName}
                pending={pending}
                submitLabel="Save name"
                onSave={async (name) => {
                  await rename(person.id, name);
                  setEditing(null);
                }}
                onCancel={() => setEditing(null)}
              />
            )}
            {archiving === person.id && (
              <div
                className="confirmation"
                role="group"
                aria-label={`Confirm archive ${person.displayName}`}
              >
                <p>
                  Archive {person.displayName}? Their books and history will be
                  preserved. They will be removed from the person picker.
                </p>
                <div className="actions">
                  <button
                    disabled={pending}
                    onClick={() => {
                      void archive(person.id)
                        .then(() => setArchiving(null))
                        .catch(() => {});
                    }}
                  >
                    Confirm archive
                  </button>
                  <button
                    className="quiet"
                    disabled={pending}
                    onClick={() => setArchiving(null)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      <h2>Add a person</h2>
      <PersonForm pending={pending} submitLabel="Add person" onSave={create} />
    </section>
  );
}
