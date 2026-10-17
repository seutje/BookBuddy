import { FieldManager } from "../custom-fields/FieldManager";
import { ImportWizard } from "../import/ImportWizard";

export function Settings({ dataDirectory }: { dataDirectory: string | null }) {
  return (
    <section>
      <h1>Settings</h1>
      <FieldManager />
      <ImportWizard />
      <section className="personal-panel">
        <h2>Your local data</h2>
        <p>
          Books and reading information stay on this computer. Household people
          are labels, with no separate accounts.
        </p>
        <label>
          Data folder
          <input readOnly value={dataDirectory ?? "Unavailable"} />
        </label>
        <p className="muted">
          Keep this folder when troubleshooting. Do not copy a database while
          BookBuddy is running.
        </p>
      </section>
    </section>
  );
}
