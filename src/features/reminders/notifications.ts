import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { DueReminder } from "./types";

export async function showReminderNotification(due: DueReminder): Promise<void> {
  let allowed = await isPermissionGranted();
  if (!allowed) {
    const permission = await requestPermission();
    allowed = permission === "granted";
  }

  if (!allowed) {
    console.error("[reminders] notification permission was not granted");
    return;
  }

  const noteLine = due.reminder.noteTitle
    ? `\n来自：${due.reminder.noteTitle}`
    : "";

  sendNotification({
    title: due.missed ? "错过的提醒" : "提醒时间到了",
    body: `${due.reminder.message}${noteLine}`,
  });
}
