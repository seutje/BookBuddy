import { invoke } from "@tauri-apps/api/core";
import type { CustomFilter, CustomDisplay } from "./custom-fields";

export const readingChoices = [
  { value: "want_to_read", label: "Want to Read" },
  { value: "on_hold", label: "On Hold" },
  { value: "currently_borrowed", label: "Currently Borrowed" },
  { value: "read", label: "Read" },
  { value: "reading", label: "Reading" },
  { value: "did_not_finish", label: "Did Not Finish" },
] as const;
export const ratingChoices = [
  { value: "loved", label: "Loved" },
  { value: "liked", label: "Liked" },
  { value: "meh", label: "Meh" },
  { value: "nope", label: "Nope" },
] as const;
export const listChoices = [
  { value: "library", label: "Library" },
  { value: "wishlist", label: "Wishlist" },
  { value: "borrowed", label: "Borrowed" },
] as const;
export type ReadingStatus = (typeof readingChoices)[number]["value"];
export type Rating = (typeof ratingChoices)[number]["value"];
export type ListStatus = (typeof listChoices)[number]["value"];
export interface BookDraft {
  title: string;
  subtitle: string | null;
  authors: string | null;
  isbn13: string | null;
  publisher: string | null;
  publicationYear: number | null;
  language: string | null;
  series: string | null;
  volume: string | null;
  description: string | null;
  notes: string | null;
  owned: boolean | null;
}
export interface Book extends BookDraft {
  id: string;
  createdAt: string;
  updatedAt: string;
  readCount: number;
  lastReadOn: string | null;
}
export interface PersonalDraft {
  readingStatus: ReadingStatus | null;
  rating: Rating | null;
  listStatus: ListStatus | null;
  notes: string | null;
}
export interface Membership extends PersonalDraft {
  id: string;
  personId: string;
  bookId: string;
  personName: string;
  archivedAt: string | null;
  readCount: number;
  lastReadOn: string | null;
}
export interface BookDetail {
  book: Book;
  memberships: Membership[];
  rowFields: CustomDisplay[];
}
export type Filter<T> =
  { mode: "any" } | { mode: "unknown" } | { mode: "equals"; value: T };
export interface BookFilter {
  personId: string | null;
  search: string;
  readingStatus: Filter<ReadingStatus>;
  rating: Filter<Rating>;
  listStatus: Filter<ListStatus>;
  owned: Filter<boolean>;
  sort:
    | "recently_added"
    | "title"
    | "author"
    | "rating"
    | "most_recently_read"
    | "reading_count";
  customFilters: CustomFilter[];
  hasReadings?: boolean | null;
  readFrom?: string | null;
  readTo?: string | null;
}
export const emptyBook: BookDraft = {
  title: "",
  subtitle: null,
  authors: null,
  isbn13: null,
  publisher: null,
  publicationYear: null,
  language: null,
  series: null,
  volume: null,
  description: null,
  notes: null,
  owned: null,
};
export const allBooksFilter: BookFilter = {
  personId: null,
  search: "",
  readingStatus: { mode: "any" },
  rating: { mode: "any" },
  listStatus: { mode: "any" },
  owned: { mode: "any" },
  sort: "recently_added",
  customFilters: [],
};
export const booksApi = {
  list: (filter: BookFilter) => invoke<BookDetail[]>("list_books", { filter }),
  get: (id: string) => invoke<BookDetail>("get_book", { id }),
  create: (draft: BookDraft, personId: string | null) =>
    invoke<string>("create_book", { draft, personId }),
  update: (id: string, draft: BookDraft) =>
    invoke<void>("update_book", { id, draft }),
  ensure: (personId: string, bookId: string) =>
    invoke<void>("ensure_person_book", { personId, bookId }),
  personal: (personId: string, bookId: string, draft: PersonalDraft) =>
    invoke<void>("update_person_book", { personId, bookId, draft }),
  markRead: (personId: string, bookId: string) =>
    invoke<void>("mark_read", { personId, bookId }),
  hints: (draft: BookDraft, excludeId: string | null = null) =>
    invoke<Book[]>("duplicate_book_hints", { draft, excludeId }),
};
export function choiceLabel(
  value: string | null,
  choices: readonly { value: string; label: string }[],
) {
  return choices.find((choice) => choice.value === value)?.label ?? "Not set";
}
