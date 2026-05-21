import { useMemo, useState } from "react";
import { ReminderAlert } from "./ReminderAlert";
import { openNoteInEditor } from "../features/windows/api";
import { closeCurrentWindow } from "../features/windows/controls";
import type { DueReminder } from "../features/reminders/types";
import { loadReminders, saveReminders } from "../features/reminders/storage";
import {
  completeReminder,
  snoozeReminder,
} from "../features/reminders/scheduler";
import { stopReminderSound } from "../features/reminders/sound";

interface ReminderAlarmWindowProps {
  reminderId: string;
}

export function ReminderAlarmWindow({ reminderId }: ReminderAlarmWindowProps) {
  const [reminders, setReminders] = useState(() => loadReminders());
  const reminder = useMemo(
    () => reminders.find((item) => item.id === reminderId) ?? null,
    [reminderId, reminders],
  );

  const due: DueReminder | null = reminder
    ? {
        reminder,
        missed: Date.now() - new Date(reminder.remindAt).getTime() > 30_000,
      }
    : null;

  const closeWindow = () => {
    void closeCurrentWindow().catch(() => undefined);
  };

  const handleComplete = () => {
    void stopReminderSound();
    const next = completeReminder(loadReminders(), reminderId);
    saveReminders(next);
    setReminders(next);
    closeWindow();
  };

  const handleSnooze = () => {
    if (!reminder) return;
    void stopReminderSound();
    const next = snoozeReminder(loadReminders(), reminder, 5);
    saveReminders(next);
    setReminders(next);
    closeWindow();
  };

  const handleOpenNote = () => {
    if (!reminder?.noteId) return;
    void openNoteInEditor(reminder.noteId).catch(() => undefined);
  };

  return (
    <div className="relative noise-bg h-screen bg-cloud text-ink overflow-hidden">
      {due ? (
        <ReminderAlert
          due={due}
          inline
          onComplete={handleComplete}
          onSnooze={handleSnooze}
          onOpenNote={handleOpenNote}
        />
      ) : (
        <div className="h-full flex items-center justify-center px-6 text-[13px] text-ink-ghost">
          提醒已处理
        </div>
      )}
    </div>
  );
}
