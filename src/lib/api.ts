import { invoke } from "@tauri-apps/api/core";

export interface AppError {
  code: "validation" | "not_found" | "storage" | "startup";
  message: string;
}

export interface Person {
  id: string;
  displayName: string;
  archivedAt: string | null;
  createdAt: string;
}

export interface Household {
  people: Person[];
  currentPersonId: string | null;
}

export interface StartupStatus {
  ready: boolean;
  error: AppError | null;
  dataDirectory: string | null;
}

export function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof error.message === "string" &&
    ["validation", "not_found", "storage", "startup"].includes(
      String(error.code),
    )
  ) {
    return error.message;
  }
  return "BookBuddy could not complete this operation. Close and reopen the app, then try again.";
}

export const api = {
  startup: () => invoke<StartupStatus>("startup_status"),
  household: () => invoke<Household>("list_people"),
  createPerson: (name: string) => invoke<void>("create_person", { name }),
  renamePerson: (id: string, name: string) =>
    invoke<void>("rename_person", { id, name }),
  archivePerson: (id: string) => invoke<void>("archive_person", { id }),
  selectPerson: (id: string | null) => invoke<void>("select_person", { id }),
};
