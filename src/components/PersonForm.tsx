import { useState } from "react";
import type { FormEvent } from "react";

interface Props {
  initialName?: string;
  pending: boolean;
  submitLabel: string;
  onSave: (name: string) => Promise<void>;
  onCancel?: () => void;
}

export function PersonForm({
  initialName = "",
  pending,
  submitLabel,
  onSave,
  onCancel,
}: Props) {
  const [name, setName] = useState(initialName);
  async function submit(event: FormEvent) {
    event.preventDefault();
    try {
      await onSave(name);
      setName("");
    } catch {
      // The shared household action reports the structured backend error.
    }
  }
  return (
    <form
      onSubmit={(event) => {
        void submit(event);
      }}
      className="person-form"
    >
      <label>
        Display name
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={100}
          required
          disabled={pending}
          autoComplete="off"
        />
      </label>
      <div className="actions">
        <button type="submit" disabled={pending || !name.trim()}>
          {pending ? "Saving…" : submitLabel}
        </button>
        {onCancel && (
          <button
            type="button"
            className="quiet"
            disabled={pending}
            onClick={onCancel}
          >
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}
