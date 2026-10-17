import { invoke } from "@tauri-apps/api/core";
import type { Book, BookDraft, Rating, ReadingStatus } from "./books";
export interface RawCell {
  kind: string;
  value: string | number | boolean | null;
  calendarDate: string | null;
  formula: string | null;
}
export interface ImportWarning {
  code: string;
  sheet: string | null;
  row: number | null;
  column: string | null;
  message: string;
}
export type PersonMapping =
  { mode: "new"; name: string } | { mode: "existing"; id: string };
export interface PeopleMapping {
  carly: PersonMapping;
  mom: PersonMapping | null;
}
export interface HeaderOverride {
  sheet: string;
  sourceHeader: string;
  destinationHeader: string | null;
}
export type BookTarget =
  | { mode: "new" }
  | { mode: "existing"; bookId: string }
  | { mode: "catalogue"; rowKey: string }
  | { mode: "unresolved" }
  | { mode: "skip" };
export interface RowDecision {
  rowKey: string;
  target: BookTarget;
  acceptConfirmedTitle: boolean;
}
export interface ImportFieldMapping {
  sourceKey: string;
  existingFieldId: string | null;
  newLabel: string;
}
export interface ImportReviewDraft {
  selectionToken: string;
  sourceOriginId: string | null;
  people: PeopleMapping;
  headers: HeaderOverride[];
  decisions: RowDecision[];
  fields: ImportFieldMapping[];
  dateOrder: "review_ambiguous" | "day_month_year" | "month_day_year";
  acknowledgeWarnings: boolean;
  acknowledgeMissingSheets: boolean;
}
export interface SourceDate {
  value: string | null;
  precision: string;
  raw: RawCell | null;
}
export interface SourceRow {
  key: string;
  sheet: string;
  rowNumber: number;
  fingerprint: string;
  occurrence: number;
  raw: Record<string, RawCell>;
}
export interface CatalogueRow {
  source: SourceRow;
  book: BookDraft;
  readingStatus: ReadingStatus | null;
  carlyRating: Rating | null;
  momRating: Rating | null;
  favourite: boolean | null;
  renewedToDeath: boolean | null;
  format: string | null;
  buyStatus: string | null;
  readingEffort: string | null;
  ageRange: string | null;
  borrowCount: number | null;
  firstSourceDate: SourceDate;
  latestSourceDate: SourceDate;
  dateType: string | null;
  originalTitle: string | null;
  checkTitle: boolean | null;
  candidates: Book[];
  target: BookTarget;
  warnings: ImportWarning[];
  errors: string[];
  fillFields: string[];
  preserveFields: string[];
}
export interface BorrowRow {
  source: SourceRow;
  title: string;
  author: string | null;
  format: string | null;
  publicationYear: number | null;
  sourceDate: SourceDate;
  dateType: string | null;
  sourcePdf: string | null;
  catalogueCandidates: string[];
  candidates: Book[];
  target: BookTarget;
  warnings: ImportWarning[];
  errors: string[];
}
export interface TitleReviewRow {
  source: SourceRow;
  workingTitle: string;
  author: string | null;
  rawPdfTitle: string | null;
  reason: string | null;
  confirmedTitle: string | null;
  catalogueCandidates: string[];
  candidates: Book[];
  target: BookTarget;
  acceptConfirmedTitle: boolean;
}
export interface SheetPreview {
  name: string;
  canonicalName: string | null;
  rowCount: number;
  headerRow: number | null;
  headers: string[];
  mapping: HeaderOverride[];
  ignored: boolean;
}
export interface ProposedField {
  sourceKey: string;
  label: string;
  scope: "book" | "person_book";
  kind: "select" | "text" | "boolean";
  choices: string[];
  existingFieldId: string | null;
}
export interface ImportCounts {
  catalogue: number;
  historicalLoans: number;
  titleReviews: number;
  ownedYes: number;
  currentlyBorrowed: number;
  favouritesYes: number;
  momMemberships: number;
}
export interface ImportPreview {
  draft: ImportReviewDraft;
  sourceName: string;
  sourceHash: string;
  reviewHash: string;
  date1904: boolean;
  sheets: SheetPreview[];
  catalogue: CatalogueRow[];
  history: BorrowRow[];
  titleReviews: TitleReviewRow[];
  fields: ProposedField[];
  counts: ImportCounts;
  warnings: ImportWarning[];
  blockingErrors: string[];
  missingSheets: string[];
  requiresTitleDecisions: number;
  preservedManualValues: number;
}
export const importApi = {
  choose: () => invoke<ImportPreview | null>("choose_library_xlsx"),
  preview: (draft: ImportReviewDraft) =>
    invoke<ImportPreview>("preview_library_xlsx_mapping", { draft }),
};
export const sheetHeaders: Record<string, readonly string[]> = {
  "Book Catalogue": [
    "Title",
    "Author",
    "Format",
    "Published",
    "Reading Status",
    "Owned?",
    "Carly Rating",
    "Mom Rating",
    "Favourite?",
    "Buy Status",
    "Borrow Count",
    "First Library Date",
    "Latest Library Date",
    "Date Type",
    "Reading Effort",
    "Age Range",
    "Renewed to Death?",
    "Notes",
    "Source / Original Title",
    "Check Title?",
  ],
  "Library History": [
    "Title",
    "Author",
    "Format",
    "Publication Year",
    "Approximate Borrow Date",
    "Actual Date Type",
    "Source PDF",
  ],
  "Check Titles": [
    "Working Title",
    "Author",
    "Exactly as PDF shows",
    "Reason",
    "Confirmed Full Title (edit here)",
  ],
};
