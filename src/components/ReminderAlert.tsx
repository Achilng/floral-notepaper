import type { DueReminder } from "../features/reminders/types";

interface ReminderAlertProps {
  due: DueReminder;
  onComplete: () => void;
  onSnooze: () => void;
  onOpenNote: () => void;
  inline?: boolean;
  compact?: boolean;
}

function formatFullTime(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function reminderTitle(due: DueReminder): string {
  return (
    due.reminder.noteTitle?.trim() ||
    due.reminder.message.trim() ||
    "提醒时间到了"
  );
}

function reminderDetail(due: DueReminder): string {
  const message = due.reminder.message.trim();
  if (due.reminder.noteTitle?.trim() && message) return message;
  return due.missed ? "错过的提醒" : "提醒时间到了";
}

export function ReminderAlert({
  due,
  onComplete,
  onSnooze,
  onOpenNote,
  inline = false,
  compact = false,
}: ReminderAlertProps) {
  const { reminder } = due;
  const wrapperClassName = inline
    ? "absolute inset-0 z-30 flex items-stretch justify-center p-2"
    : "fixed inset-0 z-[10000] flex items-center justify-center bg-ink/10 backdrop-blur-[2px]";
  const panelClassName = inline
    ? "w-full h-full rounded-xl border border-paper-deep/45 bg-cloud/95 shadow-[0_12px_30px_rgba(26,26,24,0.16)] overflow-hidden animate-menu-enter flex flex-col"
    : "w-[360px] max-w-[calc(100vw-32px)] rounded-xl border border-paper-deep/45 bg-cloud shadow-[0_18px_50px_rgba(26,26,24,0.18)] overflow-hidden animate-menu-enter";

  return (
    <div className={wrapperClassName}>
      <div className={panelClassName}>
        <div
          className={`${
            compact ? "px-3 py-2.5" : "px-5 py-4"
          } border-b border-paper-deep/25 bg-paper/55`}
        >
          <div className="flex items-start justify-between gap-3">
            <h2
              className={`${
                compact ? "text-[12px]" : "text-[14px]"
              } min-w-0 truncate font-display font-semibold text-ink`}
            >
              {reminderTitle(due)}
            </h2>
            <span className="shrink-0 text-[10px] text-ink-ghost font-mono">
              {formatFullTime(reminder.remindAt)}
            </span>
          </div>
        </div>

        <div className={`${compact ? "px-3 py-2" : "px-5 py-4"} space-y-2 flex-1`}>
          <p
            className={`${
              compact ? "text-[11px]" : "text-[13px]"
            } leading-relaxed text-ink-soft whitespace-pre-wrap`}
          >
            {reminderDetail(due)}
          </p>
          {reminder.noteTitle && !compact && (
            <div className="rounded-lg bg-paper-warm/55 border border-paper-deep/25 px-3 py-2">
              <div className="text-[10px] text-ink-ghost mb-0.5">绑定笔记</div>
              <div className="text-[12px] text-ink-soft truncate">
                {reminder.noteTitle}
              </div>
            </div>
          )}
        </div>

        <div
          className={`flex items-center justify-end gap-2 ${
            compact ? "px-3 py-2 flex-wrap" : "px-5 py-3"
          } border-t border-paper-deep/25 bg-paper/35`}
        >
          {reminder.noteId && (
            <button
              type="button"
              onClick={onOpenNote}
              className={`${
                compact ? "h-7 px-2 text-[11px]" : "h-8 px-3 text-[12px]"
              } rounded-lg border border-paper-deep/45 text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer`}
            >
              打开笔记
            </button>
          )}
          <button
            type="button"
            onClick={onSnooze}
            className={`${
              compact ? "h-7 px-2 text-[11px]" : "h-8 px-3 text-[12px]"
            } rounded-lg border border-paper-deep/45 text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer`}
          >
            稍后 5 分钟
          </button>
          <button
            type="button"
            onClick={onComplete}
            className={`${
              compact ? "h-7 px-2 text-[11px]" : "h-8 px-3 text-[12px]"
            } rounded-lg text-cloud bg-bamboo hover:bg-bamboo-light transition-colors cursor-pointer`}
          >
            完成
          </button>
        </div>
      </div>
    </div>
  );
}
