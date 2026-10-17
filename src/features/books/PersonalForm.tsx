import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { PersonalDraft } from "../../lib/books";
import { readingChoices, ratingChoices, listChoices } from "../../lib/books";
import { ChoiceSelect } from "../../components/ChoiceSelect";
export function PersonalForm({
  initial,
  name,
  pending,
  onSave,
}: {
  initial: PersonalDraft;
  name: string;
  pending: boolean;
  onSave: (draft: PersonalDraft) => Promise<unknown>;
}) {
  const [draft, setDraft] = useState<PersonalDraft>(() => ({
    readingStatus: initial.readingStatus,
    rating: initial.rating,
    listStatus: initial.listStatus,
    notes: initial.notes,
  }));
  useEffect(() => {
    setDraft((previous) => ({
      ...previous,
      readingStatus: initial.readingStatus,
    }));
  }, [initial.readingStatus]);
  async function submit(event: FormEvent) {
    event.preventDefault();
    try {
      await onSave(draft);
    } catch {
      /* Parent displays the error. */
    }
  }
  return (
    <form
      onSubmit={(event) => {
        void submit(event);
      }}
      className="book-form"
    >
      <div className="form-grid">
        <ChoiceSelect
          label="Reading status"
          value={draft.readingStatus}
          options={readingChoices}
          disabled={pending}
          onChange={(readingStatus) => setDraft({ ...draft, readingStatus })}
        />
        <ChoiceSelect
          label="Rating"
          value={draft.rating}
          options={ratingChoices}
          unknownLabel="Unknown"
          disabled={pending}
          onChange={(rating) => setDraft({ ...draft, rating })}
        />
        <ChoiceSelect
          label="Personal list"
          value={draft.listStatus}
          options={listChoices}
          disabled={pending}
          onChange={(listStatus) => setDraft({ ...draft, listStatus })}
        />
      </div>
      <label>
        {name}'s notes
        <textarea
          value={draft.notes ?? ""}
          maxLength={100000}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, notes: event.target.value })
          }
        />
      </label>
      <button disabled={pending}>
        {pending ? "Saving…" : `Save ${name}'s reading information`}
      </button>
    </form>
  );
}
