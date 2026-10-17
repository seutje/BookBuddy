import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { readingsApi } from "../../lib/readings";
import { errorMessage } from "../../lib/api";

export function QuickReading({
  bookId,
  title,
  personId,
  name,
}: {
  bookId: string;
  title: string;
  personId: string;
  name: string;
}) {
  const client = useQueryClient();
  const [lastId, setLastId] = useState<string | null>(null);
  const [undo, setUndo] = useState(false);
  const action = useMutation({
    mutationFn: async (type: "log" | "undo") => {
      if (type === "log") {
        const id = await readingsApi.logToday(bookId, [personId]);
        setLastId(id);
        setUndo(false);
      } else if (lastId) {
        await readingsApi.delete(lastId);
        setLastId(null);
        setUndo(false);
      }
    },
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["readings"] }),
        client.invalidateQueries({ queryKey: ["books"] }),
        client.invalidateQueries({ queryKey: ["book", bookId] }),
      ]);
    },
  });
  return (
    <div className="quick-reading">
      <button
        disabled={action.isPending}
        aria-label={`Log ${title} read today for ${name}`}
        onClick={() => {
          void action.mutateAsync("log").catch(() => {});
        }}
      >
        Log read today
      </button>
      <span className="muted"> For {name}; sets status to Read.</span>
      {action.error && <p role="alert">{errorMessage(action.error)}</p>}
      {lastId && (
        <p role="status">
          Reading logged today.{" "}
          <button
            className="quiet"
            disabled={action.isPending}
            onClick={() => setUndo(true)}
          >
            Undo this reading
          </button>
        </p>
      )}
      {undo && (
        <div
          className="confirmation"
          role="group"
          aria-label="Undo reading confirmation"
        >
          <p>
            Remove the reading just logged for {name}? Their status and notes
            will be preserved.
          </p>
          <div className="actions">
            <button
              disabled={action.isPending}
              onClick={() => {
                void action.mutateAsync("undo").catch(() => {});
              }}
            >
              Confirm undo reading
            </button>
            <button
              className="quiet"
              disabled={action.isPending}
              onClick={() => setUndo(false)}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
