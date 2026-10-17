import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { fieldsApi, displayCustom } from "../../lib/custom-fields";
import type {
  CustomValue,
  FieldValue,
  ValueTarget,
} from "../../lib/custom-fields";
import { errorMessage } from "../../lib/api";
import { CustomValueInput } from "./CustomValueInput";

function FieldEditor({
  field,
  pending,
  onSave,
}: {
  field: FieldValue;
  pending: boolean;
  onSave: (id: string, value: CustomValue) => Promise<unknown>;
}) {
  const [value, setValue] = useState(field.value);
  return (
    <form
      className="field-value-form"
      onSubmit={(event) => {
        event.preventDefault();
        void onSave(field.definition.id, value).catch(() => {});
      }}
    >
      <CustomValueInput
        field={field.definition}
        label={field.definition.label}
        value={value}
        onChange={setValue}
        disabled={pending}
      />
      {field.definition.description && <p>{field.definition.description}</p>}
      {field.value === null && field.definition.defaultValue !== null && (
        <p className="muted">
          Suggested default:{" "}
          {displayCustom(field.definition, field.definition.defaultValue)}. No
          value is saved until you choose and save one.
        </p>
      )}
      <button className="quiet" disabled={pending}>
        {pending ? "Saving…" : `Save ${field.definition.label}`}
      </button>
    </form>
  );
}
export function CustomFields({ target }: { target: ValueTarget }) {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["custom-values", target],
    queryFn: () => fieldsApi.values(target),
  });
  const action = useMutation({
    mutationFn: ({ id, value }: { id: string; value: CustomValue }) =>
      fieldsApi.set(target, id, value),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ["custom-values", target] }),
        client.invalidateQueries({ queryKey: ["fields"] }),
        client.invalidateQueries({ queryKey: ["books"] }),
      ]);
    },
  });
  if (query.isPending) return <p role="status">Opening attributes…</p>;
  if (query.error) return <p role="alert">{errorMessage(query.error)}</p>;
  const visible = query.data.filter((field) => field.definition.showDetail);
  if (!visible.length) return null;
  return (
    <section
      className="custom-fields"
      aria-label={
        target.scope === "book"
          ? "Shared custom attributes"
          : "Personal custom attributes"
      }
    >
      <h3>
        {target.scope === "book" ? "Shared attributes" : "Personal attributes"}
      </h3>
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {visible.map((field) => (
        <FieldEditor
          key={`${field.definition.id}-${field.definition.revision}-${JSON.stringify(field.value)}`}
          field={field}
          pending={action.isPending}
          onSave={(id, value) => action.mutateAsync({ id, value })}
        />
      ))}
    </section>
  );
}
