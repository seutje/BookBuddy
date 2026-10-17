import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { api, errorMessage } from "../../lib/api";
import type { Person } from "../../lib/api";
import {
  booksApi,
  allBooksFilter,
  choiceLabel,
  ratingChoices,
  readingChoices,
} from "../../lib/books";
import { fieldsApi } from "../../lib/custom-fields";
import { importApi, sheetHeaders } from "../../lib/import";
import type {
  BookTarget,
  CatalogueRow,
  HeaderOverride,
  ImportPreview,
  ImportReviewDraft,
  PersonMapping,
  RowDecision,
  SourceRow,
} from "../../lib/import";

function PersonMappingInput({
  label,
  value,
  people,
  optional = false,
  onChange,
}: {
  label: string;
  value: PersonMapping | null;
  people: Person[];
  optional?: boolean;
  onChange: (value: PersonMapping | null) => void;
}) {
  const selected =
    value?.mode === "existing"
      ? `existing:${value.id}`
      : (value?.mode ?? "none");
  return (
    <div>
      <label>
        {label} maps to
        <select
          value={selected}
          onChange={(event) => {
            const selected = event.target.value;
            onChange(
              selected === "none"
                ? null
                : selected === "new"
                  ? { mode: "new", name: label }
                  : {
                      mode: "existing",
                      id: selected.slice("existing:".length),
                    },
            );
          }}
        >
          <option value="new">Create a local person</option>
          {optional && (
            <option value="none">
              No person (only when ratings are blank)
            </option>
          )}
          {people
            .filter((person) => !person.archivedAt)
            .map((person) => (
              <option key={person.id} value={`existing:${person.id}`}>
                {person.displayName}
              </option>
            ))}
        </select>
      </label>
      {value?.mode === "new" && (
        <label>
          {label} local name
          <input
            value={value.name}
            maxLength={100}
            onChange={(event) =>
              onChange({ mode: "new", name: event.target.value })
            }
          />
        </label>
      )}
    </div>
  );
}
function SourceCells({ source }: { source: SourceRow }) {
  return (
    <details>
      <summary>Original source cells · row {source.rowNumber}</summary>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Column</th>
              <th>Original value</th>
              <th>Cell type</th>
              <th>Formula (cached value used)</th>
            </tr>
          </thead>
          <tbody>
            {Object.entries(source.raw).map(([header, cell]) => (
              <tr key={header}>
                <th scope="row">{header}</th>
                <td className="preserve-text">
                  {cell.value === null
                    ? "Blank"
                    : cell.value === ""
                      ? "Empty text"
                      : String(cell.value)}
                </td>
                <td>{cell.kind.replaceAll("_", " ")}</td>
                <td>{cell.formula ?? ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </details>
  );
}
function TargetPicker({
  rowKey,
  target,
  catalogue,
  existing,
  catalogueRow = false,
  onChange,
}: {
  rowKey: string;
  target: BookTarget;
  catalogue: CatalogueRow[];
  existing: { id: string; title: string; authors: string | null }[];
  catalogueRow?: boolean;
  onChange: (target: BookTarget) => void;
}) {
  const value =
    target.mode === "existing"
      ? `existing:${target.bookId}`
      : target.mode === "catalogue"
        ? `catalogue:${target.rowKey}`
        : target.mode;
  return (
    <label>
      {catalogueRow ? "Catalogue decision" : "Link source row"}
      <select
        value={value}
        onChange={(event) => {
          const selected = event.target.value;
          onChange(
            selected.startsWith("existing:")
              ? { mode: "existing", bookId: selected.slice(9) }
              : selected.startsWith("catalogue:")
                ? { mode: "catalogue", rowKey: selected.slice(10) }
                : selected === "new"
                  ? { mode: "new" }
                  : selected === "skip"
                    ? { mode: "skip" }
                    : { mode: "unresolved" },
          );
        }}
      >
        <option value="unresolved">
          {catalogueRow
            ? "Review the match before importing"
            : "Keep unlinked for review"}
        </option>
        <option value="new">Create a separate shared book</option>
        <option value="skip">Skip row · retain source provenance</option>
        {!catalogueRow && (
          <optgroup label="Source catalogue">
            {catalogue
              .filter((row) => row.source.key !== rowKey)
              .map((row) => (
                <option
                  key={row.source.key}
                  value={`catalogue:${row.source.key}`}
                >
                  Source row {row.source.rowNumber}: {row.book.title}
                </option>
              ))}
          </optgroup>
        )}
        <optgroup label="Existing household books">
          {existing.map((book) => (
            <option key={book.id} value={`existing:${book.id}`}>
              {book.title}
              {book.authors ? ` — ${book.authors}` : ""}
            </option>
          ))}
        </optgroup>
      </select>
    </label>
  );
}
function targetName(target: BookTarget) {
  return target.mode === "new"
    ? "New shared book"
    : target.mode === "existing"
      ? "Match to existing book"
      : target.mode === "catalogue"
        ? "Link to source catalogue"
        : target.mode === "skip"
          ? "Skip, keeping provenance"
          : "Needs review / unlinked";
}

export function ImportWizard() {
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [draft, setDraft] = useState<ImportReviewDraft | null>(null);
  const [dirty, setDirty] = useState(false);
  const people = useQuery({
    queryKey: ["household"],
    queryFn: api.household,
    enabled: !!preview,
  });
  const books = useQuery({
    queryKey: ["books", allBooksFilter],
    queryFn: () => booksApi.list(allBooksFilter),
    enabled: !!preview,
  });
  const fields = useQuery({
    queryKey: ["fields"],
    queryFn: fieldsApi.list,
    enabled: !!preview,
  });
  const action = useMutation({
    mutationFn: (
      work: { type: "choose" } | { type: "preview"; draft: ImportReviewDraft },
    ) =>
      work.type === "choose"
        ? importApi.choose()
        : importApi.preview(work.draft),
    onSuccess: (preview) => {
      setPreview(preview);
      setDraft(preview?.draft ?? null);
      setDirty(false);
    },
  });
  function change(next: ImportReviewDraft) {
    setDraft(next);
    setDirty(true);
  }
  function decision(
    rowKey: string,
    target: BookTarget,
    acceptConfirmedTitle = false,
  ) {
    if (!draft) return;
    const next: RowDecision = { rowKey, target, acceptConfirmedTitle };
    change({
      ...draft,
      decisions: [
        ...draft.decisions.filter((decision) => decision.rowKey !== rowKey),
        next,
      ],
    });
  }
  function chosen(rowKey: string, fallback: BookTarget) {
    return (
      draft?.decisions.find((decision) => decision.rowKey === rowKey)?.target ??
      fallback
    );
  }
  function header(mapping: HeaderOverride) {
    if (draft)
      change({
        ...draft,
        headers: [
          ...draft.headers.filter(
            (current) =>
              current.sheet !== mapping.sheet ||
              current.sourceHeader !== mapping.sourceHeader,
          ),
          mapping,
        ],
      });
  }
  const existing = books.data?.map((row) => row.book) ?? [];
  return (
    <section className="personal-panel" aria-labelledby="import-heading">
      <div className="heading-row">
        <h2 id="import-heading">Import Excel library</h2>
        <button
          disabled={action.isPending}
          onClick={() => {
            setPreview(null);
            setDraft(null);
            void action.mutateAsync({ type: "choose" }).catch(() => {});
          }}
        >
          {action.isPending ? "Inspecting…" : "Choose Excel workbook"}
        </button>
      </div>
      <p>
        Choose the family-library workbook from this computer. Inspection and
        preview leave your library unchanged.
      </p>
      {action.error && (
        <p role="alert" className="error">
          {errorMessage(action.error)}
        </p>
      )}
      {preview && draft && (
        <fieldset className="plain-fieldset" disabled={action.isPending}>
          <h3>{preview.sourceName}</h3>
          <p role="status">{`${preview.counts.catalogue} catalogue rows · ${preview.counts.historicalLoans} historical borrowing rows · ${preview.counts.titleReviews} title review rows`}</p>
          <p>{`${preview.counts.ownedYes} owned · ${preview.counts.currentlyBorrowed} currently borrowed · ${preview.counts.favouritesYes} favourite flags · ${preview.counts.momMemberships} Mom ratings`}</p>
          <p>
            Borrowing dates are unverified historical source dates and may be
            return-date proxies. They do not create reading sessions. Blank
            ratings remain Unknown.
          </p>
          <table>
            <caption>Detected worksheets</caption>
            <thead>
              <tr>
                <th>Sheet</th>
                <th>Detected rows</th>
                <th>Treatment</th>
              </tr>
            </thead>
            <tbody>
              {preview.sheets.map((sheet) => (
                <tr key={sheet.name}>
                  <th scope="row">{sheet.name}</th>
                  <td>{sheet.rowCount}</td>
                  <td>
                    {sheet.ignored
                      ? "Reference only · no book records"
                      : `Header row ${sheet.headerRow ?? "unknown"}`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {preview.blockingErrors.length > 0 && (
            <div role="alert" className="error">
              <h3>Review needed</h3>
              <ul>
                {preview.blockingErrors.map((error, index) => (
                  <li key={`${index}-${error}`}>{error}</li>
                ))}
              </ul>
            </div>
          )}
          <h3>People mapping</h3>
          <div className="form-grid">
            <PersonMappingInput
              label="Carly"
              value={draft.people.carly}
              people={people.data?.people ?? []}
              onChange={(value) => {
                if (value)
                  change({
                    ...draft,
                    people: { ...draft.people, carly: value },
                  });
              }}
            />
            <PersonMappingInput
              label="Mom"
              optional
              value={draft.people.mom}
              people={people.data?.people ?? []}
              onChange={(mom) =>
                change({ ...draft, people: { ...draft.people, mom } })
              }
            />
          </div>
          <p>
            Carly's status and rating stay separate from Mom's rating. A blank
            Mom rating creates no personal book membership.
          </p>
          <label>
            Slash-date interpretation
            <select
              value={draft.dateOrder}
              onChange={(event) => {
                const dateOrder = event.target.value;
                if (
                  dateOrder === "review_ambiguous" ||
                  dateOrder === "day_month_year" ||
                  dateOrder === "month_day_year"
                )
                  change({ ...draft, dateOrder });
              }}
            >
              <option value="review_ambiguous">
                Leave ambiguous dates unknown for review
              </option>
              <option value="day_month_year">
                Day / month / year (explicit choice)
              </option>
              <option value="month_day_year">
                Month / day / year (explicit choice)
              </option>
            </select>
          </label>
          <details>
            <summary>Adjust column mappings</summary>
            {preview.sheets
              .filter((sheet) => !sheet.ignored)
              .map((sheet) => (
                <section key={sheet.name}>
                  <h4>{sheet.name}</h4>
                  <div className="form-grid">
                    {sheet.mapping.map((mapping) => {
                      const override = draft.headers.find(
                        (item) =>
                          item.sheet === mapping.sheet &&
                          item.sourceHeader === mapping.sourceHeader,
                      );
                      return (
                        <label key={mapping.sourceHeader}>
                          {mapping.sourceHeader}
                          <select
                            value={
                              (override ?? mapping).destinationHeader ?? ""
                            }
                            onChange={(event) =>
                              header({
                                ...mapping,
                                destinationHeader: event.target.value || null,
                              })
                            }
                          >
                            <option value="">
                              Preserve in provenance only
                            </option>
                            {(sheetHeaders[mapping.sheet] ?? []).map(
                              (header) => (
                                <option key={header} value={header}>
                                  {header}
                                </option>
                              ),
                            )}
                          </select>
                        </label>
                      );
                    })}
                  </div>
                </section>
              ))}
          </details>
          <details>
            <summary>Shared and personal attribute mappings</summary>
            <p>
              Definitions are available on all applicable records without
              bulk-filling values. Existing populated values are preserved.
            </p>
            {preview.fields.map((field) => {
              const mapping = draft.fields.find(
                (mapping) => mapping.sourceKey === field.sourceKey,
              ) ?? {
                sourceKey: field.sourceKey,
                existingFieldId: field.existingFieldId,
                newLabel: field.label,
              };
              function update(next: typeof mapping) {
                if (draft)
                  change({
                    ...draft,
                    fields: [
                      ...draft.fields.filter(
                        (current) => current.sourceKey !== field.sourceKey,
                      ),
                      next,
                    ],
                  });
              }
              return (
                <div className="field-filter" key={field.sourceKey}>
                  <strong>
                    {field.label} ·{" "}
                    {field.scope === "book"
                      ? "Shared book"
                      : "Each person's book"}
                  </strong>
                  <label>
                    Use attribute
                    <select
                      value={mapping.existingFieldId ?? "new"}
                      onChange={(event) =>
                        update({
                          ...mapping,
                          existingFieldId:
                            event.target.value === "new"
                              ? null
                              : event.target.value,
                        })
                      }
                    >
                      <option value="new">Create an attribute</option>
                      {fields.data
                        ?.filter(
                          (definition) =>
                            definition.active &&
                            definition.scope === field.scope &&
                            definition.kind === field.kind,
                        )
                        .map((definition) => (
                          <option key={definition.id} value={definition.id}>
                            {definition.label}
                          </option>
                        ))}
                    </select>
                  </label>
                  {mapping.existingFieldId === null && (
                    <label>
                      New attribute label
                      <input
                        value={mapping.newLabel}
                        maxLength={100}
                        onChange={(event) =>
                          update({ ...mapping, newLabel: event.target.value })
                        }
                      />
                    </label>
                  )}
                  {field.choices.length > 0 && (
                    <p>Source choices: {field.choices.join(", ")}</p>
                  )}
                </div>
              );
            })}
          </details>
          <details>
            <summary>{`${preview.warnings.length} source warnings and reconciliation notes`}</summary>
            <ul>
              {preview.warnings.map((warning, index) => (
                <li key={index}>
                  {warning.sheet}
                  {warning.row ? ` row ${warning.row}` : ""}
                  {warning.column ? ` · ${warning.column}` : ""}:{" "}
                  {warning.message}
                </li>
              ))}
            </ul>
          </details>
          <h3>Catalogue decisions</h3>
          <p>
            {preview.requiresTitleDecisions} rows need an explicit match
            decision. Approximate title guesses never merge books. Existing
            populated shared and personal values are preserved; imported blanks
            never erase them.
          </p>
          {preview.catalogue.map((row) => (
            <details className="import-row" key={row.source.key}>
              <summary>
                {row.source.rowNumber}. {row.book.title || "Missing title"} ·{" "}
                {targetName(chosen(row.source.key, row.target))}
              </summary>
              <TargetPicker
                rowKey={row.source.key}
                target={chosen(row.source.key, row.target)}
                catalogue={preview.catalogue}
                existing={existing}
                catalogueRow
                onChange={(target) => decision(row.source.key, target)}
              />
              {row.candidates.length > 0 && (
                <p>
                  Possible existing entries:{" "}
                  {row.candidates
                    .map(
                      (book) =>
                        `${book.title}${book.authors ? ` (${book.authors})` : ""}`,
                    )
                    .join("; ")}
                  . Review the edition before matching.
                </p>
              )}
              <dl className="metadata">
                <div>
                  <dt>Author</dt>
                  <dd>{row.book.authors ?? "Unknown"}</dd>
                </div>
                <div>
                  <dt>Published</dt>
                  <dd>{row.book.publicationYear ?? "Unknown"}</dd>
                </div>
                <div>
                  <dt>Household owns it</dt>
                  <dd>
                    {row.book.owned === null
                      ? "Unknown"
                      : row.book.owned
                        ? "Yes"
                        : "No"}
                  </dd>
                </div>
                <div>
                  <dt>Carly's status</dt>
                  <dd>{choiceLabel(row.readingStatus, readingChoices)}</dd>
                </div>
                <div>
                  <dt>Carly's rating</dt>
                  <dd>
                    {row.carlyRating === null
                      ? "Unknown"
                      : choiceLabel(row.carlyRating, ratingChoices)}
                  </dd>
                </div>
                <div>
                  <dt>Mom's rating</dt>
                  <dd>
                    {row.momRating === null
                      ? "Unknown"
                      : choiceLabel(row.momRating, ratingChoices)}
                  </dd>
                </div>
              </dl>
              {row.fillFields.length > 0 && (
                <p>Fill empty values: {row.fillFields.join(", ")}</p>
              )}
              {row.preserveFields.length > 0 && (
                <p>Preserve existing values: {row.preserveFields.join(", ")}</p>
              )}
              {row.errors.length > 0 && (
                <ul className="error">
                  {row.errors.map((error) => (
                    <li key={error}>{error}</li>
                  ))}
                </ul>
              )}
              <SourceCells source={row.source} />
            </details>
          ))}
          <details>
            <summary>{`${preview.history.length} historical borrowing records`}</summary>
            {preview.history.map((row) => (
              <details className="import-row" key={row.source.key}>
                <summary>
                  {row.source.rowNumber}. {row.title || "Missing source title"}{" "}
                  · {targetName(chosen(row.source.key, row.target))}
                </summary>
                <TargetPicker
                  rowKey={row.source.key}
                  target={chosen(row.source.key, row.target)}
                  catalogue={preview.catalogue}
                  existing={existing}
                  onChange={(target) => decision(row.source.key, target)}
                />
                <p>
                  Historical source date: {row.sourceDate.value ?? "Unknown"} ·{" "}
                  {row.sourceDate.precision} precision ·{" "}
                  {row.dateType ?? "Unverified source interpretation"}
                </p>
                <p>Source PDF label: {row.sourcePdf ?? "Unknown"}</p>
                <p>
                  Each source row remains a separate loan record, including
                  identical-looking repeats.
                </p>
                <SourceCells source={row.source} />
              </details>
            ))}
          </details>
          <details>
            <summary>{`${preview.titleReviews.length} title verification tasks`}</summary>
            {preview.titleReviews.map((row) => {
              const selected = chosen(row.source.key, row.target);
              const accept =
                draft.decisions.find(
                  (decision) => decision.rowKey === row.source.key,
                )?.acceptConfirmedTitle ?? row.acceptConfirmedTitle;
              return (
                <details className="import-row" key={row.source.key}>
                  <summary>
                    {row.source.rowNumber}. {row.workingTitle} ·{" "}
                    {row.reason ?? "Needs verification"}
                  </summary>
                  <TargetPicker
                    rowKey={row.source.key}
                    target={selected}
                    catalogue={preview.catalogue}
                    existing={existing}
                    onChange={(target) =>
                      decision(row.source.key, target, accept)
                    }
                  />
                  <p>Original PDF wording: {row.rawPdfTitle ?? "Unknown"}</p>
                  <p>
                    Confirmed title in source:{" "}
                    {row.confirmedTitle ?? "Not provided"}
                  </p>
                  {row.confirmedTitle && (
                    <label className="checkbox">
                      <input
                        type="checkbox"
                        checked={accept}
                        disabled={
                          selected.mode === "unresolved" ||
                          selected.mode === "skip"
                        }
                        onChange={(event) =>
                          decision(
                            row.source.key,
                            selected,
                            event.target.checked,
                          )
                        }
                      />
                      Use this confirmed source title for the linked shared book
                    </label>
                  )}
                  <SourceCells source={row.source} />
                </details>
              );
            })}
          </details>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={draft.acknowledgeWarnings}
              onChange={(event) =>
                change({ ...draft, acknowledgeWarnings: event.target.checked })
              }
            />
            I reviewed the source warnings, date interpretation and borrow-count
            differences
          </label>
          {preview.missingSheets.some(
            (sheet) => sheet === "Library History" || sheet === "Check Titles",
          ) && (
            <label className="checkbox">
              <input
                type="checkbox"
                checked={draft.acknowledgeMissingSheets}
                onChange={(event) =>
                  change({
                    ...draft,
                    acknowledgeMissingSheets: event.target.checked,
                  })
                }
              />
              Continue with the listed source-data sheets missing
            </label>
          )}
          {dirty && (
            <p role="status" className="notice">
              Mappings changed. Refresh the preview to review the resulting
              changes.
            </p>
          )}
          <div className="actions">
            <button
              disabled={action.isPending}
              onClick={() => {
                void action
                  .mutateAsync({ type: "preview", draft })
                  .catch(() => {});
              }}
            >
              Refresh import preview
            </button>
            <button
              className="quiet"
              disabled={action.isPending}
              onClick={() => {
                setPreview(null);
                setDraft(null);
                setDirty(false);
              }}
            >
              Close preview
            </button>
          </div>
        </fieldset>
      )}
    </section>
  );
}
