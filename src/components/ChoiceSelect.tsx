interface Props<T extends string> {
  label: string;
  value: T | null;
  options: readonly { value: T; label: string }[];
  onChange: (value: T | null) => void;
  disabled?: boolean;
  unknownLabel?: string;
}
export function ChoiceSelect<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
  unknownLabel = "Not set",
}: Props<T>) {
  return (
    <label>
      {label}
      <select
        value={value ?? ""}
        disabled={disabled}
        onChange={(event) =>
          onChange(
            options.find((option) => option.value === event.target.value)
              ?.value ?? null,
          )
        }
      >
        <option value="">{unknownLabel}</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
