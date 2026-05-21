export type ReminderType = "countdown" | "alarm";

export type ReminderRepeat = "none" | "daily" | "weekly";

export interface Reminder {
  id: string;
  type: ReminderType;
  message: string;
  remindAt: string;
  repeat: ReminderRepeat;
  noteId?: string;
  noteTitle?: string;
  ringtonePath?: string;
  done: boolean;
  createdAt: string;
  updatedAt: string;
  lastTriggeredAt?: string;
  notifiedAt?: string;
}

export interface ReminderDraft {
  type: ReminderType;
  message: string;
  remindAt: Date;
  repeat: ReminderRepeat;
  noteId?: string;
  noteTitle?: string;
  ringtonePath?: string;
}

export interface DueReminder {
  reminder: Reminder;
  missed: boolean;
}

export interface BoundReminderNote {
  id: string;
  title: string;
}
