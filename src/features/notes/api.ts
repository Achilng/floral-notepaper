import { invoke } from "@tauri-apps/api/core";
import i18n from "i18next";
import type { Note, NoteMetadata, SaveNoteRequest } from "./types";

export function listNotes(): Promise<NoteMetadata[]> {
  return invoke("notes_list");
}

export function getNote(id: string): Promise<Note> {
  return invoke("notes_get", { id });
}

export function createNote(request: SaveNoteRequest): Promise<Note> {
  return invoke("notes_create", { request });
}

export function updateNote(id: string, request: SaveNoteRequest): Promise<Note> {
  return invoke("notes_update", { id, request });
}

export function deleteNote(id: string): Promise<void> {
  return invoke("notes_delete", { id });
}

export function moveNoteCategory(id: string, category: string): Promise<NoteMetadata> {
  return invoke("notes_move_category", { id, category });
}

export function listCategories(): Promise<string[]> {
  return invoke("categories_list");
}

export function createCategory(name: string): Promise<void> {
  return invoke("categories_create", { name });
}

export function renameCategory(oldName: string, newName: string): Promise<void> {
  return invoke("categories_rename", { oldName, newName });
}

export function deleteCategory(name: string): Promise<void> {
  return invoke("categories_delete", { name });
}

export function readExternalFile(path: string): Promise<string> {
  return invoke("read_external_file", { path });
}

export function saveExternalFile(path: string, content: string): Promise<void> {
  return invoke("save_external_file", { path, content });
}

export function getFileModifiedTime(path: string): Promise<number> {
  return invoke("get_file_modified_time", { path });
}

type BackendError = {
  code?: unknown;
  message?: unknown;
  details?: unknown;
};

type Translate = (key: string, options?: Record<string, unknown>) => string;

const ERROR_CODES_WITH_DETAILS = new Set(["categoryAlreadyExists", "categoryNotFound"]);

function fieldLabel(field: unknown, translate: Translate): string | undefined {
  if (field === "globalShortcut") {
    return translate("settings.quickNoteShortcut");
  }

  return typeof field === "string" ? field : undefined;
}

function categoryFromMessage(message: string): string | undefined {
  return message.match(/分类「(.+?)」/)?.[1] ?? message.match(/Category "(.+?)"/)?.[1];
}

function localizeBackendError(code: string, message: string, details: unknown, translate: Translate) {
  const options: Record<string, unknown> = {};

  if (details && typeof details === "object") {
    const detailMap = details as Record<string, unknown>;
    if (typeof detailMap.category === "string") {
      options.category = detailMap.category;
    }

    const label = fieldLabel(detailMap.field, translate);
    if (label) {
      options.field = label;
    }
  }

  if (!options.category && ERROR_CODES_WITH_DETAILS.has(code)) {
    options.category = categoryFromMessage(message);
  }

  const key = `errors.${code}`;
  const translated = translate(key, options);
  return translated === key ? message : translated;
}

export function getErrorMessage(
  error: unknown,
  translate: Translate = i18n.t.bind(i18n) as Translate,
): string {
  if (typeof error === "string") {
    const match = error.match(/^([A-Za-z0-9_]+):\s*(.*)$/);
    if (!match) return error;
    return localizeBackendError(match[1], match[2], undefined, translate);
  }

  if (error && typeof error === "object") {
    const backendError = error as BackendError;

    if (typeof backendError.code === "string" && typeof backendError.message === "string") {
      return localizeBackendError(
        backendError.code,
        backendError.message,
        backendError.details,
        translate,
      );
    }

    if ("message" in backendError) {
      return String(backendError.message);
    }
  }

  return "操作失败";
}
