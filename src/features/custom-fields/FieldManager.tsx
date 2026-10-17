import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fieldsApi,
  fieldDraft,
  newField,
  fieldKinds,
  fieldScopes,
  emptySettings,
} from "../../lib/custom-fields";
import type {
  ChangePreview,
  FieldDefinition,
  FieldDraft,
  FieldKind,
} from "../../lib/custom-fields";
import { errorMessage } from "../../lib/api";
import { ChoiceSelect } from "../../components/ChoiceSelect";
import { CustomValueInput } from "./CustomValueInput";

function DefinitionEditor({
  initial,
  existing,
  pending,
  onSave,
  onCancel,
}: {
  initial: FieldDraft;
  existing?: FieldDefinition;
  pending: boolean;
  onSave: (draft: FieldDraft) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState(() => fieldDraft(initial));
  function kindChanged(kind: FieldKind) {
    setDraft({
      ...draft,
      kind,
      settings: { ...emptySettings },
      defaultValue: null,
      options: draft.options.map((option) => ({ ...option, active: false })),
    });
  }
  function check(
    key: "active" | "showDetail" | "showInRows" | "showInFilters",
    label: string,
  ) {
    return (
      <label className="checkbox">
        <input
          autoFocus
          type="checkbox"
          checked={draft[key]}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, [key]: event.target.checked })
          }
        />
        {label}
      </label>
    );
  }
  return (
    <form
      className="book-form"
      onSubmit={(event) => {
        event.preventDefault();
        void onSave(draft).catch(() => {});
      }}
    >
      <label>
        Attribute name
        <input
          required
          maxLength={100}
          value={draft.label}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, label: event.target.value })
          }
        />
      </label>
      <div className="form-grid">
        <ChoiceSelect
          label="Applies to"
          value={draft.scope}
          options={fieldScopes}
          disabled={pending || (existing?.valueCount ?? 0) > 0}
          onChange={(scope) => {
            if (scope) setDraft({ ...draft, scope });
          }}
        />
        <ChoiceSelect
          label="Attribute type"
          value={draft.kind}
          options={fieldKinds}
          disabled={pending}
          onChange={(kind) => {
            if (kind) kindChanged(kind);
          }}
        />
      </div>
      {existing && existing.valueCount > 0 && (
        <p>
          {existing.valueCount} saved values. Scope is fixed. Type changes
          require an impact review.
        </p>
      )}
      <label>
        Description
        <textarea
          value={draft.description}
          maxLength={10000}
          disabled={pending}
          onChange={(event) =>
            setDraft({ ...draft, description: event.target.value })
          }
        />
      </label>
      {(draft.kind === "number" || draft.kind === "rating") && (
        <div className="form-grid">
          {(["min", "max"] as const).map((key) => (
            <label key={key}>
              {key === "min" ? "Minimum" : "Maximum"}
              <input
                type="number"
                step={draft.kind === "rating" ? 1 : "any"}
                value={draft.settings[key] ?? ""}
                disabled={pending}
                placeholder={
                  draft.kind === "rating"
                    ? key === "min"
                      ? "1"
                      : "5"
                    : "No limit"
                }
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    settings: {
                      ...draft.settings,
                      [key]: event.target.value
                        ? Number(event.target.value)
                        : null,
                    },
                  })
                }
              />
            </label>
          ))}
          <label>
            Unit label
            <input
              value={draft.settings.unit ?? ""}
              maxLength={100}
              disabled={pending}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  settings: {
                    ...draft.settings,
                    unit: event.target.value || null,
                  },
                })
              }
            />
          </label>
        </div>
      )}
      {draft.kind === "text" && (
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.settings.multiline}
            disabled={pending}
            onChange={(event) =>
              setDraft({
                ...draft,
                settings: {
                  ...draft.settings,
                  multiline: event.target.checked,
                },
              })
            }
          />
          Allow multiple lines
        </label>
      )}
      {draft.kind === "select" && (
        <fieldset>
          <legend>Choices</legend>
          {draft.options.map((option, index) => (
            <div className="choice-row" key={option.id}>
              <label>
                Choice {index + 1} label
                <input
                  required
                  value={option.label}
                  maxLength={100}
                  disabled={pending}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      options: draft.options.map((item) =>
                        item.id === option.id
                          ? { ...item, label: event.target.value }
                          : item,
                      ),
                    })
                  }
                />
              </label>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={option.active}
                  disabled={pending}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      options: draft.options.map((item) =>
                        item.id === option.id
                          ? { ...item, active: event.target.checked }
                          : item,
                      ),
                    })
                  }
                />
                Available for new selections
              </label>
            </div>
          ))}
          <button
            type="button"
            className="quiet"
            disabled={pending || draft.options.length >= 200}
            onClick={() =>
              setDraft({
                ...draft,
                options: [
                  ...draft.options,
                  { id: crypto.randomUUID(), label: "", active: true },
                ],
              })
            }
          >
            Add choice
          </button>
          <p className="muted">
            Retiring a choice preserves its saved values. Renaming preserves the
            choice's identity.
          </p>
        </fieldset>
      )}
      <CustomValueInput
        field={draft}
        label="Display-only default"
        value={draft.defaultValue}
        onChange={(defaultValue) => setDraft({ ...draft, defaultValue })}
        disabled={pending}
      />
      <p className="muted">
        Defaults are suggestions. Existing and new records remain Unknown until
        a value is explicitly saved.
      </p>
      <div className="visibility-options">
        {check("active", "Active attribute")}
        {check("showDetail", "Show in book details")}
        {check("showInRows", "Show in Library rows")}
        {check("showInFilters", "Show in filters")}
      </div>
      <div className="actions">
        <button disabled={pending}>
          {pending
            ? "Preparing…"
            : existing
              ? "Review attribute changes"
              : "Create attribute"}
        </button>
        <button
          type="button"
          className="quiet"
          disabled={pending}
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}

export function FieldManager() {
  const query = useQuery({ queryKey: ["fields"], queryFn: fieldsApi.list });
  const client = useQueryClient();
  const [editing, setEditing] = useState<FieldDefinition | "new" | null>(null);
  const [review, setReview] = useState<{
    id: string;
    draft: FieldDraft;
    impact: ChangePreview;
  } | null>(null);
  const action = useMutation({
    mutationFn: async (work: () => Promise<ChangePreview | void>) => work(),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["fields"] }),
        client.invalidateQueries({ queryKey: ["custom-values"] }),
        client.invalidateQueries({ queryKey: ["books"] }),
      ]);
    },
  });
  async function save(draft: FieldDraft) {
    if (editing === "new") {
      await action.mutateAsync(async () => {
        await fieldsApi.create(draft);
      });
      setEditing(null);
      return;
    }
    if (!editing) return;
    const impact = await action.mutateAsync(() =>
      fieldsApi.preview(editing.id, draft),
    );
    if (impact) {
      setReview({ id: editing.id, draft, impact });
      setEditing(null);
    }
  }
  async function toggle(field: FieldDefinition) {
    const draft = fieldDraft(field);
    draft.active = !draft.active;
    await action.mutateAsync(async () => {
      const impact = await fieldsApi.preview(field.id, draft);
      setReview({ id: field.id, draft, impact });
    });
  }
  async function move(index: number, offset: number) {
    if (!query.data) return;
    const ids = query.data.map((field) => field.id);
    const id = ids[index];
    const other = ids[index + offset];
    if (!id || !other) return;
    ids[index] = other;
    ids[index + offset] = id;
    await action.mutateAsync(() => fieldsApi.reorder(ids));
  }
  return (
    <section aria-labelledby="custom-field-heading">
      <div className="heading-row">
        <h2 id="custom-field-heading">Custom attributes</h2>
        <button
          className="quiet"
          disabled={action.isPending}
          onClick={() => {
            setEditing("new");
            setReview(null);
            action.reset();
          }}
        >
          Add attribute
        </button>
      </div>
      <p>
        Add shared facts or each person's own attributes. Missing values stay
        Unknown.
      </p>
      {query.error && <p role="alert">{errorMessage(query.error)}</p>}
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {editing && (
        <DefinitionEditor
          key={typeof editing === "string" ? "new" : editing.id}
          initial={
            editing === "new" ? newField(query.data?.length ?? 0) : editing
          }
          existing={editing === "new" ? undefined : editing}
          pending={action.isPending}
          onSave={save}
          onCancel={() => setEditing(null)}
        />
      )}
      {review && (
        <div
          className="confirmation"
          role="group"
          aria-label="Attribute change review"
        >
          <h3>Review {review.draft.label}</h3>
          <p>
            {review.impact.savedValueCount} saved values;{" "}
            {review.impact.clearedValueCount} will be cleared.
          </p>
          {review.impact.warnings.length ? (
            <ul>
              {review.impact.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          ) : (
            <p>Saved values and choice identities are preserved.</p>
          )}
          <div className="actions">
            <button
              disabled={action.isPending}
              onClick={() => {
                void action
                  .mutateAsync(() =>
                    fieldsApi.update(
                      review.id,
                      review.draft,
                      review.impact.token,
                      review.impact.requiresConfirmation,
                    ),
                  )
                  .then(() => setReview(null))
                  .catch(() => {});
              }}
            >
              Confirm attribute change
            </button>
            <button
              className="quiet"
              disabled={action.isPending}
              onClick={() => {
                setReview(null);
                action.reset();
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
      <ul className="people-list">
        {query.data?.map((field, index) => (
          <li key={field.id}>
            <div className="person-row">
              <div>
                <strong>{field.label}</strong>
                <p className="muted">
                  {
                    fieldScopes.find((scope) => scope.value === field.scope)
                      ?.label
                  }{" "}
                  ·{" "}
                  {fieldKinds.find((kind) => kind.value === field.kind)?.label}{" "}
                  · {field.valueCount} saved values
                  {field.active ? "" : " · Hidden"}
                </p>
              </div>
              <div className="actions">
                <button
                  className="quiet"
                  disabled={action.isPending}
                  onClick={() => {
                    setEditing(field);
                    setReview(null);
                    action.reset();
                  }}
                >
                  Edit {field.label}
                </button>
                <button
                  className="quiet"
                  disabled={action.isPending}
                  onClick={() => {
                    void toggle(field).catch(() => {});
                  }}
                >
                  {field.active ? "Hide" : "Reactivate"} {field.label}
                </button>
                <button
                  className="quiet"
                  aria-label={`Move ${field.label} up`}
                  disabled={action.isPending || index === 0}
                  onClick={() => {
                    void move(index, -1).catch(() => {});
                  }}
                >
                  ↑
                </button>
                <button
                  className="quiet"
                  aria-label={`Move ${field.label} down`}
                  disabled={
                    action.isPending || index === (query.data?.length ?? 0) - 1
                  }
                  onClick={() => {
                    void move(index, 1).catch(() => {});
                  }}
                >
                  ↓
                </button>
              </div>
            </div>
          </li>
        ))}
      </ul>
      {query.isPending && <p role="status">Opening attributes…</p>}
    </section>
  );
}
