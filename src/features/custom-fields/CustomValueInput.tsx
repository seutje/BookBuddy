import type { CustomValue, FieldDraft } from "../../lib/custom-fields";

export function CustomValueInput({
  field,
  label,
  value,
  onChange,
  disabled,
  allowRetired = false,
}: {
  field: FieldDraft;
  label: string;
  value: CustomValue;
  onChange: (value: CustomValue) => void;
  disabled?: boolean;
  allowRetired?: boolean;
}) {
  if (field.kind === "boolean")
    return (
      <fieldset className="boolean-value" disabled={disabled}>
        <legend>{label}</legend>
        <div className="actions">
          {(
            [
              { value: null, text: "Unknown" },
              { value: true, text: "Yes" },
              { value: false, text: "No" },
            ] as const
          ).map((choice) => (
            <button
              key={choice.text}
              type="button"
              className="quiet"
              aria-label={`${label}: ${choice.text}`}
              aria-pressed={value === choice.value}
              onClick={() => onChange(choice.value)}
            >
              {choice.text}
            </button>
          ))}
        </div>
      </fieldset>
    );
  if (field.kind === "select")
    return (
      <label>
        {label}
        <select
          value={typeof value === "string" ? value : ""}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value || null)}
        >
          <option value="">Unknown</option>
          {field.options
            .filter(
              (option) => option.active || allowRetired || option.id === value,
            )
            .map((option) => (
              <option
                key={option.id}
                value={option.id}
                disabled={!option.active && !allowRetired}
              >
                {option.label}
                {option.active ? "" : " (retired)"}
              </option>
            ))}
        </select>
      </label>
    );
  const stringValue = value === null ? "" : String(value);
  return (
    <div className="custom-input">
      <label>
        {label}
        {field.kind === "text" && field.settings.multiline ? (
          <textarea
            value={stringValue}
            maxLength={100000}
            disabled={disabled}
            onChange={(event) => onChange(event.target.value)}
          />
        ) : (
          <input
            type={
              field.kind === "number" || field.kind === "rating"
                ? "number"
                : field.kind === "date"
                  ? "date"
                  : "text"
            }
            value={stringValue}
            placeholder="Unknown"
            disabled={disabled}
            maxLength={100000}
            min={
              field.kind === "rating"
                ? (field.settings.min ?? 1)
                : (field.settings.min ?? undefined)
            }
            max={
              field.kind === "rating"
                ? (field.settings.max ?? 5)
                : (field.settings.max ?? undefined)
            }
            step={field.kind === "rating" ? 1 : "any"}
            onChange={(event) => {
              if (field.kind === "number" || field.kind === "rating")
                onChange(
                  event.target.value === "" ||
                    !Number.isFinite(event.target.valueAsNumber)
                    ? null
                    : event.target.valueAsNumber,
                );
              else
                onChange(
                  field.kind === "date" && event.target.value === ""
                    ? null
                    : event.target.value,
                );
            }}
          />
        )}
      </label>
      <p className="muted">
        {value === null
          ? "Unknown"
          : value === ""
            ? "Empty text (saved separately from Unknown)"
            : (field.settings.unit ?? "")}
      </p>
      <div className="actions">
        {field.kind === "text" && value === null && (
          <button
            type="button"
            className="quiet"
            disabled={disabled}
            onClick={() => onChange("")}
          >
            Set empty text for {label}
          </button>
        )}
        {value !== null && (
          <button
            type="button"
            className="quiet"
            disabled={disabled}
            onClick={() => onChange(null)}
          >
            Clear {label} to Unknown
          </button>
        )}
      </div>
    </div>
  );
}
