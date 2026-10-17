import { invoke } from "@tauri-apps/api/core";
export interface BackupSettings {
  enabled: boolean;
  keep: number;
}
export interface BackupStatus {
  settings: BackupSettings;
  automaticCount: number;
  automaticProblem: boolean;
  backupDirectory: string;
}
export interface Counts {
  books: number;
  people: number;
  memberships: number;
  fields: number;
  readings: number;
  historicalLoans: number;
  importBatches: number;
  sourceRows: number;
  titleTasks: number;
}
export interface Manifest {
  format: string;
  version: number;
  schemaVersion: number;
  appVersion: string;
  createdAt: string;
  databaseSha256: string;
  databaseBytes: number;
  coverAssets: string[];
}
export interface RestorePreview {
  token: string;
  sourceName: string;
  manifest: Manifest;
  backupCounts: Counts;
  currentCounts: Counts;
  reviewHash: string;
}
export const backupsApi = {
  status: () => invoke<BackupStatus>("backup_status"),
  settings: (settings: BackupSettings) =>
    invoke<void>("update_backup_settings", { settings }),
  export: () => invoke<Manifest | null>("export_library_backup"),
  choose: () => invoke<RestorePreview | null>("choose_library_backup"),
  refresh: (token: string) =>
    invoke<RestorePreview>("refresh_restore_preview", { token }),
  restore: (token: string, reviewHash: string, confirmReplace: boolean) =>
    invoke<string>("confirm_library_restore", {
      token,
      reviewHash,
      confirmReplace,
    }),
  close: () => invoke<void>("close_after_restore"),
};
