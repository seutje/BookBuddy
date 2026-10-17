import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { errorMessage } from "../../lib/api";
import { backupsApi } from "../../lib/backups";
import type { BackupSettings, Counts, RestorePreview } from "../../lib/backups";

const countLabels: Record<keyof Counts, string> = {
  books: "Shared books",
  people: "People (including archived)",
  memberships: "Personal book memberships",
  fields: "Custom field definitions",
  readings: "Actual reading sessions",
  historicalLoans: "Historical loans",
  importBatches: "Import batches",
  sourceRows: "Original source rows",
  titleTasks: "Title verification tasks",
};
export function BackupPanel() {
  const client = useQueryClient();
  const status = useQuery({
    queryKey: ["backupStatus"],
    queryFn: backupsApi.status,
  });
  const [settings, setSettings] = useState<BackupSettings | null>(null);
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [notice, setNotice] = useState("");
  const action = useMutation({
    mutationFn: async (
      kind: "export" | "choose" | "refresh" | "settings" | "restore",
    ) => {
      setNotice("");
      if (kind === "export") {
        const exported = await backupsApi.export();
        if (exported)
          setNotice(
            "Backup exported. Save a copy on another device or drive for recovery.",
          );
      }
      if (kind === "choose") {
        setPreview(null);
        setConfirmed(false);
        setPreview(await backupsApi.choose());
      }
      if (kind === "refresh" && preview) {
        setConfirmed(false);
        setPreview(await backupsApi.refresh(preview.token));
      }
      if (kind === "settings" && settings) {
        await backupsApi.settings(settings);
        setSettings(null);
        await client.invalidateQueries({ queryKey: ["backupStatus"] });
        setNotice("Automatic backup settings saved.");
      }
      if (kind === "restore" && preview && confirmed) {
        const recovery = await backupsApi.restore(
          preview.token,
          preview.reviewHash,
          confirmed,
        );
        client.setQueryData(["restored"], recovery);
      }
    },
  });
  const current = settings ?? status.data?.settings;
  return (
    <section className="personal-panel" aria-labelledby="backups-heading">
      <h2 id="backups-heading">Backups and restore</h2>
      <p>
        Export a complete local library, including personal information, custom
        fields, reading sessions, historical loans and import evidence. Backups
        are unencrypted files; store them somewhere you trust.
      </p>
      {(action.error || status.error) && (
        <p role="alert" className="error">
          {errorMessage(action.error ?? status.error)}
        </p>
      )}
      {notice && (
        <p role="status" className="notice">
          {notice}
        </p>
      )}
      <fieldset className="plain-fieldset" disabled={action.isPending}>
        <div className="actions">
          <button onClick={() => action.mutate("export")}>Export backup</button>
          <button onClick={() => action.mutate("choose")}>
            Choose backup to restore
          </button>
        </div>
        {action.isPending && (
          <p role="status">Working with your local backup…</p>
        )}
        {status.data && current && (
          <>
            <h3>Automatic local backups</h3>
            {status.data.automaticProblem && (
              <p role="alert" className="error">
                The last automatic backup could not finish. Check free disk
                space and folder access, then save these settings to retry.
              </p>
            )}
            <label className="checkbox">
              <input
                type="checkbox"
                checked={current.enabled}
                onChange={(event) =>
                  setSettings({ ...current, enabled: event.target.checked })
                }
              />
              Back up on startup, at most once every 24 hours
            </label>
            <label>
              Automatic backups to keep
              <input
                type="number"
                min={1}
                max={20}
                value={current.keep}
                onChange={(event) =>
                  setSettings({ ...current, keep: Number(event.target.value) })
                }
              />
            </label>
            <button
              disabled={
                !settings ||
                !Number.isInteger(settings.keep) ||
                settings.keep < 1 ||
                settings.keep > 20
              }
              onClick={() => action.mutate("settings")}
            >
              Save backup settings
            </button>
            <p>
              {status.data.automaticCount} automatic backups saved. Rotation
              preserves manual and pre-import/pre-restore recovery files.
            </p>
            <label>
              Backup folder
              <input readOnly value={status.data.backupDirectory} />
            </label>
          </>
        )}
        {preview && (
          <section className="notice">
            <h3>Review replacement restore</h3>
            <p>
              {preview.sourceName} · created {preview.manifest.createdAt} ·
              schema {preview.manifest.schemaVersion}
            </p>
            <p>
              This replaces the entire current library. It does not merge
              records. Changes made after this backup will be absent from the
              restored library. A complete pre-restore recovery backup is saved
              first.
            </p>
            <table>
              <caption>Current library and selected backup</caption>
              <thead>
                <tr>
                  <th>Records</th>
                  <th>Current</th>
                  <th>Backup</th>
                </tr>
              </thead>
              <tbody>
                {(Object.keys(countLabels) as (keyof Counts)[]).map((key) => (
                  <tr key={key}>
                    <th scope="row">{countLabels[key]}</th>
                    <td>{preview.currentCounts[key]}</td>
                    <td>{preview.backupCounts[key]}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={confirmed}
                onChange={(event) => setConfirmed(event.target.checked)}
              />
              Replace my entire current library with this backup
            </label>
            <div className="actions">
              <button
                disabled={!confirmed}
                onClick={() => action.mutate("restore")}
              >
                Confirm replacement restore
              </button>
              <button onClick={() => action.mutate("refresh")}>
                Refresh restore preview
              </button>
              <button
                onClick={() => {
                  setPreview(null);
                  setConfirmed(false);
                }}
              >
                Cancel restore
              </button>
            </div>
          </section>
        )}
      </fieldset>
    </section>
  );
}
