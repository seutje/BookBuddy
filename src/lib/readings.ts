import { invoke } from "@tauri-apps/api/core";
export interface Participant {
  personId: string;
  displayName: string;
  archivedAt: string | null;
}
export interface ReadingEvent {
  id: string;
  bookId: string;
  title: string;
  occurredOn: string;
  note: string | null;
  createdAt: string;
  updatedAt: string;
  participants: Participant[];
}
export interface ReadingDraft {
  bookId: string;
  occurredOn: string;
  participantIds: string[];
  note: string | null;
  setRead: boolean;
}
export interface ReadingFilter {
  personId: string | null;
  bookId: string | null;
  from: string | null;
  to: string | null;
}
export const readingsApi = {
  today: () => invoke<string>("local_today_date"),
  list: (filter: ReadingFilter) =>
    invoke<ReadingEvent[]>("list_readings", { filter }),
  log: (draft: ReadingDraft) => invoke<string>("log_reading", { draft }),
  logToday: (bookId: string, participantIds: string[]) =>
    invoke<string>("log_reading_today", { bookId, participantIds }),
  edit: (id: string, draft: ReadingDraft) =>
    invoke<void>("edit_reading", { id, draft }),
  delete: (id: string) => invoke<void>("delete_reading", { id }),
};
