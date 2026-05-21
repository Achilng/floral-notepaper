import type { DueReminder, Reminder, ReminderDraft } from "./types";

export const REMINDER_CHECK_INTERVAL_MS = 1_000;
export const DEFAULT_REMINDER_MESSAGE = "提醒时间到了";

const MISSED_THRESHOLD_MS = 30_000;

function nextId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function createReminder(draft: ReminderDraft): Reminder {
  const now = new Date().toISOString();
  const message = draft.message.trim() || DEFAULT_REMINDER_MESSAGE;

  return {
    id: nextId(),
    type: draft.type,
    message,
    remindAt: draft.remindAt.toISOString(),
    repeat: draft.repeat,
    noteId: draft.noteId,
    noteTitle: draft.noteTitle,
    ringtonePath: draft.ringtonePath,
    done: false,
    createdAt: now,
    updatedAt: now,
  };
}

export function createCountdownReminder(
  minutes: number,
  draft: Omit<ReminderDraft, "type" | "remindAt">,
): Reminder {
  return createCountdownReminderSeconds(Math.round(Math.max(1, minutes) * 60), draft);
}

export function createCountdownReminderSeconds(
  seconds: number,
  draft: Omit<ReminderDraft, "type" | "remindAt">,
): Reminder {
  const remindAt = new Date(Date.now() + Math.max(1, Math.round(seconds)) * 1000);
  return createReminder({ ...draft, type: "countdown", remindAt });
}

function nextRepeatDate(reminder: Reminder, now: Date): Date {
  const next = new Date(reminder.remindAt);
  const stepDays = reminder.repeat === "weekly" ? 7 : 1;

  do {
    next.setDate(next.getDate() + stepDays);
  } while (next.getTime() <= now.getTime());

  return next;
}

export function collectDueReminders(
  reminders: Reminder[],
  now = new Date(),
): { due: DueReminder[]; reminders: Reminder[] } {
  const due: DueReminder[] = [];
  const nextReminders = reminders.map((reminder) => {
    if (reminder.done || new Date(reminder.remindAt).getTime() > now.getTime()) {
      return reminder;
    }
    if (reminder.notifiedAt === reminder.remindAt) {
      return reminder;
    }

    due.push({
      reminder,
      missed: now.getTime() - new Date(reminder.remindAt).getTime() > MISSED_THRESHOLD_MS,
    });

    return {
      ...reminder,
      notifiedAt: reminder.remindAt,
      updatedAt: now.toISOString(),
    };
  });

  return { due, reminders: nextReminders };
}

export function completeReminder(reminders: Reminder[], id: string): Reminder[] {
  const now = new Date();
  const updatedAt = now.toISOString();
  return reminders.map((reminder) => {
    if (reminder.id !== id) return reminder;
    if (reminder.repeat === "none") {
      return { ...reminder, done: true, updatedAt };
    }
    return {
      ...reminder,
      remindAt: nextRepeatDate(reminder, now).toISOString(),
      lastTriggeredAt: undefined,
      notifiedAt: undefined,
      done: false,
      updatedAt,
    };
  });
}

export function snoozeReminder(
  reminders: Reminder[],
  source: Reminder,
  minutes: number,
): Reminder[] {
  const updatedAt = new Date().toISOString();
  const remindAt = new Date(Date.now() + minutes * 60_000).toISOString();
  const exists = reminders.some((reminder) => reminder.id === source.id);
  const snoozed: Reminder = {
    ...source,
    repeat: "none",
    remindAt,
    done: false,
    lastTriggeredAt: undefined,
    notifiedAt: undefined,
    updatedAt,
  };

  return exists
    ? reminders.map((reminder) => (reminder.id === source.id ? snoozed : reminder))
    : [snoozed, ...reminders];
}
