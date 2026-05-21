import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Reminder } from "./types";

const RINGTONE_KEY = "floral-notepaper.reminderRingtonePath";
const ringtoneFilters = [
  { name: "Audio", extensions: ["mp3", "wav", "ogg", "flac"] },
];

function canUseLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

export function getReminderRingtonePath(): string | null {
  if (!canUseLocalStorage()) return null;
  return window.localStorage.getItem(RINGTONE_KEY);
}

export function setReminderRingtonePath(path: string | null): void {
  if (!canUseLocalStorage()) return;
  if (path) {
    window.localStorage.setItem(RINGTONE_KEY, path);
  } else {
    window.localStorage.removeItem(RINGTONE_KEY);
  }
}

export async function chooseReminderRingtone(): Promise<string | null> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: ringtoneFilters,
  });

  return typeof path === "string" ? path : null;
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export async function stopReminderSound(): Promise<void> {
  try {
    await invoke("stop_reminder_sound");
  } catch (error) {
    console.error("[reminders] failed to stop reminder sound", error);
  }
}

export async function playReminderSound(reminder: Reminder): Promise<void> {
  await stopReminderSound();

  try {
    await invoke("play_reminder_sound", {
      path: reminder.ringtonePath ?? null,
    });
  } catch (error) {
    const message = errorMessage(error);
    console.error(
      `[reminders] failed to play ringtone${reminder.ringtonePath ? ` (${reminder.ringtonePath})` : ""}: ${message}`,
      error,
    );

    if (!reminder.ringtonePath) return;

    try {
      await invoke("play_reminder_sound", { path: null });
    } catch (fallbackError) {
      console.error(
        "[reminders] failed to play fallback reminder sound",
        fallbackError,
      );
    }
  }
}
