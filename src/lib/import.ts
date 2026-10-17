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
  existingLoanId: string | null;
  existingBookId: string | null;
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
  existingTaskId: string | null;
  existingState: string | null;
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
  sources: ImportSource[];
  alreadyImportedBatchId: string | null;
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
  commit: (draft: ImportReviewDraft, reviewHash: string) =>
    invoke<ImportSummary>("commit_library_xlsx_import", { draft, reviewHash }),
  batches: () => invoke<ImportBatch[]>("list_import_batches"),
  rows: (batchId: string) =>
    invoke<ImportedRow[]>("list_import_rows", { batchId }),
  tasks: (includeResolved: boolean) =>
    invoke<TitleTask[]>("list_title_verification_tasks", { includeResolved }),
  resolve: (id: string, resolution: TitleResolution) =>
    invoke<void>("resolve_title_verification", { id, resolution }),
  loans: (bookId: string | null) =>
    invoke<HistoricalLoan[]>("list_historical_loans", { bookId }),
  baseline: (bookId: string) =>
    invoke<BorrowSummary | null>("get_borrow_summary", { bookId }),
  linkLoan: (id: string, bookId: string, expectedBookId: string | null) =>
    invoke<void>("link_historical_loan", { id, bookId, expectedBookId }),
};
export interface ImportSource {
  id: string;
  label: string;
  lastFilename: string;
  carlyPersonId: string;
  momPersonId: string | null;
  createdAt: string;
}
export interface ImportSummary {
  batchId: string;
  originId: string;
  alreadyImported: boolean;
  booksAdded: number;
  booksUpdated: number;
  catalogueRowsSkipped: number;
  peopleCreated: number;
  membershipsCreated: number;
  historicalLoansAdded: number;
  historicalLoansReused: number;
  unlinkedLoans: number;
  fieldsCreated: number;
  customValuesWritten: number;
  titleTasksAdded: number;
  unresolvedTitleTasks: number;
  sourceRowsSaved: number;
  warningCount: number;
  backupName: string | null;
  carlyPersonId: string;
}
export interface ImportBatch {
  id: string;
  originId: string;
  sourceFilename: string;
  sourceHash: string;
  mappingVersion: number;
  importedAt: string;
  summaryJson: string;
}
export interface ImportedRow {
  id: string;
  sheetName: string;
  sheetRowNumber: number;
  rawJson: string;
  warningJson: string;
  bookId: string | null;
  loanId: string | null;
  verificationId: string | null;
}
export interface TitleTask {
  id: string;
  bookId: string | null;
  currentTitle: string | null;
  workingTitle: string;
  rawPdfTitle: string | null;
  reason: string | null;
  proposedConfirmedTitle: string | null;
  state: string;
  sourceRowId: string;
}
export interface TitleResolution {
  bookId: string | null;
  confirmedTitle: string | null;
  expectedCurrentTitle: string | null;
  expectedState: string;
}
export interface HistoricalLoan {
  id: string;
  bookId: string | null;
  currentTitle: string | null;
  personId: string;
  personName: string;
  sourceTitle: string;
  sourceAuthor: string | null;
  sourceFormat: string | null;
  publicationYear: number | null;
  sourceDateJson: string;
  dateInterpretation: string;
  datePrecision: string;
  sourcePdf: string | null;
  importedRowId: string;
}
export interface BorrowSummary {
  bookId: string;
  importedCountBaseline: number | null;
  firstSourceDateJson: string;
  latestSourceDateJson: string;
  sourceDateType: string | null;
  linkedHistoryCount: number;
}
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
