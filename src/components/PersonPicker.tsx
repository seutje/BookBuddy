import { useRef, useState } from "react";
import type { Person } from "../lib/api";

interface Props {
  people: Person[];
  currentId: string | null;
  pending: boolean;
  onSelect: (id: string | null) => Promise<void>;
}

export function PersonPicker({ people, currentId, pending, onSelect }: Props) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const current = people.find((person) => person.id === currentId);
  function close() {
    setOpen(false);
    requestAnimationFrame(() => trigger.current?.focus());
  }
  async function select(id: string | null) {
    try {
      await onSelect(id);
      close();
    } catch {
      /* The household action displays the error. */
    }
  }
  return (
    <div
      className="person-picker"
      onKeyDown={(event) => {
        if (open && event.key === "Escape") {
          event.preventDefault();
          close();
        }
      }}
    >
      <span>Viewing</span>
      <button
        ref={trigger}
        className="picker-trigger"
        aria-label="Current person"
        aria-expanded={open}
        aria-controls="person-choices"
        disabled={pending}
        onClick={() => setOpen(!open)}
      >
        {current?.displayName ?? "Everyone"} ▾
      </button>
      {open && (
        <div
          id="person-choices"
          className="person-choices"
          role="group"
          aria-label="Choose a person"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              close();
            }
          }}
        >
          <button
            aria-pressed={currentId === null}
            disabled={pending}
            onClick={() => {
              void select(null);
            }}
          >
            View Everyone
          </button>
          {people.map((person) => (
            <button
              key={person.id}
              aria-pressed={currentId === person.id}
              disabled={pending}
              onClick={() => {
                void select(person.id);
              }}
            >
              View {person.displayName}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
