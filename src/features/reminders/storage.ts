import { emit } from "@tauri-apps/api/event";
import type { Reminder } from "./types";

const REMINDERS_KEY = "floral-notepaper.reminders";
const REMINDER_SURFACES_KEY = "floral-notepaper.reminderSurfaces";
export const REMINDERS_CHANGED_EVENT = "floral-notepaper.reminders-changed";
export type ReminderSurfaceMode = "notepad" | "tile";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function normalizeReminder(value: unknown): Reminder | null {
  if (!isRecord(value)) return null;

  const type = value.type;
  const repeat = value.repeat;

  if (type !== "countdown" && type !== "alarm") return null;
  if (repeat !== "none" && repeat !== "daily" && repeat !== "weekly") {
    return null;
  }
  if (
    !isString(value.id) ||
    !isString(value.message) ||
    !isString(value.remindAt) ||
    !isString(value.createdAt) ||
    !isString(value.updatedAt) ||
    typeof value.done !== "boolean"
  ) {
    return null;
  }

  const reminder: Reminder = {
    id: value.id,
    type,
    message: value.message,
    remindAt: value.remindAt,
    repeat,
    done: value.done,
    createdAt: value.createdAt,
    updatedAt: value.updatedAt,
  };

  if (isString(value.noteId)) reminder.noteId = value.noteId;
  if (isString(value.noteTitle)) reminder.noteTitle = value.noteTitle;
  if (isString(value.ringtonePath)) reminder.ringtonePath = value.ringtonePath;
  if (isString(value.lastTriggeredAt)) {
    reminder.lastTriggeredAt = value.lastTriggeredAt;
  }
  if (isString(value.notifiedAt)) {
    reminder.notifiedAt = value.notifiedAt;
  }

  return reminder;
}

function canUseLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

export function loadReminders(): Reminder[] {
  if (!canUseLocalStorage()) return [];

  try {
    const raw = window.localStorage.getItem(REMINDERS_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map(normalizeReminder)
      .filter((reminder): reminder is Reminder => reminder !== null)
      .sort((left, right) => left.remindAt.localeCompare(right.remindAt));
  } catch {
    return [];
  }
}

export function saveReminders(reminders: Reminder[]): void {
  if (!canUseLocalStorage()) return;
  window.localStorage.setItem(REMINDERS_KEY, JSON.stringify(reminders));
  window.dispatchEvent(new CustomEvent(REMINDERS_CHANGED_EVENT));
  void emit("reminders-changed").catch(() => undefined);
}

export function upsertReminder(reminder: Reminder): Reminder[] {
  const reminders = loadReminders();
  const exists = reminders.some((item) => item.id === reminder.id);
  const next = exists
    ? reminders.map((item) => (item.id === reminder.id ? reminder : item))
    : [reminder, ...reminders];
  saveReminders(next);
  return next;
}

export function deleteReminder(id: string): Reminder[] {
  const next = loadReminders().filter((reminder) => reminder.id !== id);
  saveReminders(next);
  return next;
}

function loadReminderSurfaces(): Record<string, ReminderSurfaceMode> {
  if (!canUseLocalStorage()) return {};
  try {
    const raw = window.localStorage.getItem(REMINDER_SURFACES_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return {};
    const result: Record<string, ReminderSurfaceMode> = {};
    Object.entries(parsed).forEach(([noteId, mode]) => {
      if (mode === "notepad" || mode === "tile") result[noteId] = mode;
    });
    return result;
  } catch {
    return {};
  }
}

function saveReminderSurfaces(surfaces: Record<string, ReminderSurfaceMode>): void {
  if (!canUseLocalStorage()) return;
  window.localStorage.setItem(REMINDER_SURFACES_KEY, JSON.stringify(surfaces));
}

export function getReminderSurfaceMode(noteId: string | undefined): ReminderSurfaceMode | null {
  if (!noteId) return null;
  return loadReminderSurfaces()[noteId] ?? null;
}

export function setReminderSurfaceMode(
  noteId: string,
  mode: ReminderSurfaceMode,
): void {
  const surfaces = loadReminderSurfaces();
  surfaces[noteId] = mode;
  saveReminderSurfaces(surfaces);
}

export function clearReminderSurfaceMode(noteId: string | null): void {
  if (!noteId) return;
  const surfaces = loadReminderSurfaces();
  delete surfaces[noteId];
  saveReminderSurfaces(surfaces);
}
