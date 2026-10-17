import { useQuery } from "@tanstack/react-query";
import { fieldsApi, firstValue } from "../../lib/custom-fields";
import type { CustomFilter } from "../../lib/custom-fields";
import { errorMessage } from "../../lib/api";
import { CustomValueInput } from "./CustomValueInput";

export function CustomFilters({
  filters,
  onChange,
}: {
  filters: CustomFilter[];
  onChange: (filters: CustomFilter[]) => void;
}) {
  const query = useQuery({ queryKey: ["fields"], queryFn: fieldsApi.list });
  function update(next: CustomFilter) {
    onChange(
      filters.map((filter) =>
        filter.fieldId === next.fieldId ? next : filter,
      ),
    );
  }
  return (
    <fieldset className="custom-filters">
      <legend>Custom attribute filters</legend>
      {query.error && <p role="alert">{errorMessage(query.error)}</p>}
      {query.data
        ?.filter((field) => field.active && field.showInFilters)
        .map((field) => {
          const filter = filters.find((filter) => filter.fieldId === field.id);
          return (
            <div key={field.id} className="field-filter">
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={!!filter}
                  onChange={(event) =>
                    onChange(
                      event.target.checked
                        ? [
                            ...filters,
                            {
                              fieldId: field.id,
                              condition: { mode: "unknown" },
                            },
                          ]
                        : filters.filter(
                            (filter) => filter.fieldId !== field.id,
                          ),
                    )
                  }
                />
                Filter by {field.label}
              </label>
              <span className="muted">
                {field.scope === "book" ? "Shared book" : "Each person's book"}
              </span>
              {filter && (
                <>
                  <div className="actions">
                    <button
                      type="button"
                      className="quiet"
                      aria-label={`${field.label} filter: Unknown`}
                      aria-pressed={filter.condition.mode === "unknown"}
                      onClick={() =>
                        update({
                          fieldId: field.id,
                          condition: { mode: "unknown" },
                        })
                      }
                    >
                      Unknown / not set
                    </button>
                    <button
                      type="button"
                      className="quiet"
                      aria-label={`${field.label} filter: Specific value`}
                      disabled={
                        field.kind === "select" && field.options.length === 0
                      }
                      aria-pressed={filter.condition.mode === "equals"}
                      onClick={() =>
                        update({
                          fieldId: field.id,
                          condition: {
                            mode: "equals",
                            value: firstValue(field),
                          },
                        })
                      }
                    >
                      Specific value
                    </button>
                  </div>
                  {filter.condition.mode === "equals" && (
                    <CustomValueInput
                      allowRetired
                      field={field}
                      label={`${field.label} filter value`}
                      value={filter.condition.value}
                      onChange={(value) =>
                        update({
                          fieldId: field.id,
                          condition:
                            value === null
                              ? { mode: "unknown" }
                              : { mode: "equals", value },
                        })
                      }
                    />
                  )}
                </>
              )}
            </div>
          );
        })}
      {filters.length > 0 && (
        <button type="button" className="quiet" onClick={() => onChange([])}>
          Clear custom filters
        </button>
      )}
      {query.data?.every((field) => !field.active || !field.showInFilters) && (
        <p className="muted">Add attributes in Settings to filter by them.</p>
      )}
    </fieldset>
  );
}
