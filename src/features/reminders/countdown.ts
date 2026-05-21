import type { Reminder } from "./types";

export function formatRemainingTime(totalSeconds: number): string {
  const normalized = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(normalized / 3600);
  const minutes = Math.floor((normalized % 3600) / 60);
  const seconds = normalized % 60;
  return [hours, minutes, seconds]
    .map((part) => part.toString().padStart(2, "0"))
    .join(":");
}

export const formatDuration = formatRemainingTime;

export function remainingSeconds(remindAt: string, now = Date.now()): number {
  return Math.max(0, Math.ceil((new Date(remindAt).getTime() - now) / 1000));
}

export function getActiveReminderForNote(
  reminders: Reminder[],
  noteId: string | null,
  now = Date.now(),
): Reminder | null {
  if (!noteId) return null;
  return (
    reminders
      .filter(
        (reminder) =>
          !reminder.done &&
          reminder.noteId === noteId,
      )
      .sort((left, right) => {
        const leftTime = new Date(left.remindAt).getTime();
        const rightTime = new Date(right.remindAt).getTime();
        const leftOverdue = leftTime <= now;
        const rightOverdue = rightTime <= now;
        if (leftOverdue !== rightOverdue) return leftOverdue ? -1 : 1;
        return leftTime - rightTime;
      })[0] ??
    null
  );
}

export const findActiveCountdownForNote = getActiveReminderForNote;
