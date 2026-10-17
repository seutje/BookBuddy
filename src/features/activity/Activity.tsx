import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { readingsApi } from "../../lib/readings";
import type { ReadingDraft } from "../../lib/readings";
import type { Person } from "../../lib/api";
import { errorMessage } from "../../lib/api";
import { ReadingForm } from "./ReadingForm";
import { ReadingHistory } from "./ReadingHistory";

export function Activity({ current }: { current: Person | undefined }) {
  const [adding, setAdding] = useState(false);
  const [from, setFrom] = useState<string | null>(null);
  const [to, setTo] = useState<string | null>(null);
  const client = useQueryClient();
  const action = useMutation({
    mutationFn: (draft: ReadingDraft) => readingsApi.log(draft),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["readings"] }),
        client.invalidateQueries({ queryKey: ["books"] }),
        client.invalidateQueries({ queryKey: ["book"] }),
      ]);
      setAdding(false);
    },
  });
  return (
    <section>
      <div className="heading-row">
        <h1>Activity</h1>
        <button disabled={action.isPending} onClick={() => setAdding(true)}>
          Log a reading
        </button>
      </div>
      <p>
        Each entry is one reading session. A book can be read again as often as
        you like.
      </p>
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {adding && (
        <div className="empty">
          <h2>Log reading</h2>
          <ReadingForm
            key={current?.id ?? "everyone"}
            current={current}
            pending={action.isPending}
            onSave={(draft) => action.mutateAsync(draft)}
            onCancel={() => setAdding(false)}
          />
        </div>
      )}
      <div className="form-grid">
        <label>
          Read from
          <input
            type="date"
            value={from ?? ""}
            onChange={(event) => setFrom(event.target.value || null)}
          />
        </label>
        <label>
          Read through
          <input
            type="date"
            value={to ?? ""}
            onChange={(event) => setTo(event.target.value || null)}
          />
        </label>
      </div>
      <ReadingHistory
        key={current?.id ?? "everyone"}
        current={current}
        filter={{ personId: current?.id ?? null, bookId: null, from, to }}
        showRecent
      />
    </section>
  );
}
