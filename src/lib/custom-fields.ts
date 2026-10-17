import { invoke } from "@tauri-apps/api/core";

export type CustomValue = boolean | string | number | null;
export type FieldKind =
  "boolean" | "select" | "number" | "date" | "text" | "rating";
export type FieldScope = "book" | "person_book";
export interface FieldOption {
  id: string;
  label: string;
  active: boolean;
}
export interface FieldSettings {
  min: number | null;
  max: number | null;
  unit: string | null;
  multiline: boolean;
}
export interface FieldDraft {
  scope: FieldScope;
  kind: FieldKind;
  label: string;
  description: string;
  settings: FieldSettings;
  defaultValue: CustomValue;
  options: FieldOption[];
  orderIndex: number;
  active: boolean;
  showDetail: boolean;
  showInRows: boolean;
  showInFilters: boolean;
}
export interface FieldDefinition extends FieldDraft {
  id: string;
  revision: number;
  valueCount: number;
}
export interface FieldValue {
  definition: FieldDefinition;
  value: CustomValue;
}
export type ValueTarget =
  | { scope: "book"; bookId: string }
  | { scope: "person_book"; personId: string; bookId: string };
export interface ChangePreview {
  token: string;
  requiresConfirmation: boolean;
  clearedValueCount: number;
  savedValueCount: number;
  warnings: string[];
}
export interface CustomFilter {
  fieldId: string;
  condition:
    { mode: "unknown" } | { mode: "equals"; value: Exclude<CustomValue, null> };
}
export interface CustomDisplay {
  fieldId: string;
  label: string;
  personName: string | null;
  display: string;
}
export const fieldKinds = [
  { value: "boolean", label: "Yes / No" },
  { value: "select", label: "Choice" },
  { value: "number", label: "Number" },
  { value: "date", label: "Calendar date" },
  { value: "text", label: "Text" },
  { value: "rating", label: "Custom numeric rating" },
] as const;
export const fieldScopes = [
  { value: "book", label: "Shared book attribute" },
  { value: "person_book", label: "Each person's book" },
] as const;
export const emptySettings: FieldSettings = {
  min: null,
  max: null,
  unit: null,
  multiline: false,
};
export function newField(orderIndex: number): FieldDraft {
  return {
    scope: "person_book",
    kind: "boolean",
    label: "",
    description: "",
    settings: { ...emptySettings },
    defaultValue: null,
    options: [],
    orderIndex,
    active: true,
    showDetail: true,
    showInRows: false,
    showInFilters: true,
  };
}
export function fieldDraft(field: FieldDraft): FieldDraft {
  return {
    scope: field.scope,
    kind: field.kind,
    label: field.label,
    description: field.description,
    settings: { ...field.settings },
    defaultValue: field.defaultValue,
    options: field.options.map((option) => ({ ...option })),
    orderIndex: field.orderIndex,
    active: field.active,
    showDetail: field.showDetail,
    showInRows: field.showInRows,
    showInFilters: field.showInFilters,
  };
}
export function displayCustom(field: FieldDraft, value: CustomValue): string {
  if (value === null) return "Unknown";
  if (field.kind === "boolean") return value ? "Yes" : "No";
  if (field.kind === "select") {
    const option = field.options.find((option) => option.id === value);
    return option
      ? `${option.label}${option.active ? "" : " (retired)"}`
      : "Unknown choice";
  }
  if (value === "") return "Empty text";
  return `${String(value)}${field.settings.unit ? ` ${field.settings.unit}` : ""}`;
}
export function firstValue(field: FieldDraft): Exclude<CustomValue, null> {
  if (field.kind === "boolean") return false;
  if (field.kind === "select")
    return (
      field.options.find((option) => option.active)?.id ??
      field.options[0]?.id ??
      ""
    );
  if (field.kind === "number" || field.kind === "rating")
    return field.settings.min ?? (field.kind === "rating" ? 1 : 0);
  if (field.kind === "date") {
    const today = new Date();
    return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  }
  return "";
}
export const fieldsApi = {
  list: () => invoke<FieldDefinition[]>("list_custom_fields"),
  create: (draft: FieldDraft) =>
    invoke<string>("create_custom_field", { draft }),
  preview: (id: string, draft: FieldDraft) =>
    invoke<ChangePreview>("preview_custom_field_change", { id, draft }),
  update: (id: string, draft: FieldDraft, token: string, confirmed: boolean) =>
    invoke<void>("update_custom_field", { id, draft, token, confirmed }),
  reorder: (ids: string[]) => invoke<void>("reorder_custom_fields", { ids }),
  values: (target: ValueTarget) =>
    invoke<FieldValue[]>("get_custom_values", { target }),
  set: (target: ValueTarget, fieldId: string, value: CustomValue) =>
    invoke<void>("set_custom_value", { target, fieldId, value }),
};
