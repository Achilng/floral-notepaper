import { useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent } from "react";
import { SlidingButtonGroup } from "./SlidingButtonGroup";
import type {
  BoundReminderNote,
  Reminder,
  ReminderRepeat,
  ReminderType,
} from "../features/reminders/types";
import {
  createCountdownReminder,
  createCountdownReminderSeconds,
  createReminder,
  DEFAULT_REMINDER_MESSAGE,
} from "../features/reminders/scheduler";
import { formatDuration } from "../features/reminders/countdown";
import {
  chooseReminderRingtone,
  getReminderRingtonePath,
  setReminderRingtonePath,
} from "../features/reminders/sound";

interface ReminderPanelProps {
  reminders: Reminder[];
  boundNote: BoundReminderNote | null;
  onCreate: (reminder: Reminder) => void;
  onDelete: (id: string) => void;
  onComplete: (id: string) => void;
  onSnooze: (reminder: Reminder) => void;
  onOpenNote?: (noteId: string) => void;
  onClose: () => void;
  compact?: boolean;
}

const quickMinutes = [5, 15, 25, 60];

const repeatOptions: Array<{ value: ReminderRepeat; label: string }> = [
  { value: "none", label: "不重复" },
  { value: "daily", label: "每天" },
  { value: "weekly", label: "每周" },
];

const typeOptions: Array<{ value: ReminderType; label: string }> = [
  { value: "countdown", label: "倒计时" },
  { value: "alarm", label: "闹钟" },
];

function toLocalDateTimeValue(date: Date): string {
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function formatReminderTime(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function repeatLabel(repeat: ReminderRepeat): string {
  return repeatOptions.find((option) => option.value === repeat)?.label ?? "不重复";
}

const MAX_COUNTDOWN_SECONDS = 23 * 3600 + 59 * 60 + 59;

function clampCountdownSeconds(seconds: number): number {
  return Math.max(0, Math.min(MAX_COUNTDOWN_SECONDS, Math.round(seconds)));
}

function composeSeconds(hours: number, minutes: number, seconds: number): number {
  return clampCountdownSeconds(hours * 3600 + minutes * 60 + seconds);
}

interface TimeNumberInputProps {
  label: string;
  value: number;
  max: number;
  onChange: (value: number) => void;
  onIncrement: () => void;
  onDecrement: () => void;
}

function TimeNumberInput({
  label,
  value,
  max,
  onChange,
  onIncrement,
  onDecrement,
}: TimeNumberInputProps) {
  return (
    <label className="block">
      <span className="block text-[10px] text-ink-ghost mb-1">{label}</span>
      <div className="flex h-8 overflow-hidden rounded-lg border border-paper-deep/35 bg-cloud/70">
        <button
          type="button"
          onClick={onDecrement}
          className="w-7 text-[12px] text-ink-ghost hover:text-bamboo hover:bg-paper-warm cursor-pointer"
        >
          -
        </button>
        <input
          type="number"
          min={0}
          max={max}
          value={value.toString().padStart(2, "0")}
          onChange={(event) =>
            onChange(Math.max(0, Math.min(max, Number(event.target.value) || 0)))
          }
          className="min-w-0 flex-1 px-1 text-center bg-transparent text-[12px] text-ink-soft font-mono tabular-nums outline-none"
        />
        <button
          type="button"
          onClick={onIncrement}
          className="w-7 text-[12px] text-ink-ghost hover:text-bamboo hover:bg-paper-warm cursor-pointer"
        >
          +
        </button>
      </div>
    </label>
  );
}

interface CountdownDialProps {
  seconds: number;
  onChange: (seconds: number) => void;
}

function CountdownDial({ seconds, onChange }: CountdownDialProps) {
  const ref = useRef<SVGSVGElement>(null);
  const maxSeconds = 2 * 60 * 60;
  const clamped = Math.min(maxSeconds, Math.max(1, seconds));
  const angle = (clamped / maxSeconds) * Math.PI * 2 - Math.PI / 2;
  const markerX = 54 + Math.cos(angle) * 38;
  const markerY = 54 + Math.sin(angle) * 38;
  const circumference = 2 * Math.PI * 38;
  const progress = circumference * (clamped / maxSeconds);
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const second = clamped % 60;
  const hourAngle = (((hours % 12) + minutes / 60) / 12) * Math.PI * 2 - Math.PI / 2;
  const minuteAngle = ((minutes + second / 60) / 60) * Math.PI * 2 - Math.PI / 2;
  const secondAngle = (second / 60) * Math.PI * 2 - Math.PI / 2;
  const handPoint = (handAngle: number, length: number) => ({
    x: 54 + Math.cos(handAngle) * length,
    y: 54 + Math.sin(handAngle) * length,
  });
  const hourHand = handPoint(hourAngle, 20);
  const minuteHand = handPoint(minuteAngle, 30);
  const secondHand = handPoint(secondAngle, 37);

  const updateFromPoint = (clientX: number, clientY: number) => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    const x = clientX - box.left - box.width / 2;
    const y = clientY - box.top - box.height / 2;
    const raw = Math.atan2(y, x) + Math.PI / 2;
    const normalized = raw < 0 ? raw + Math.PI * 2 : raw;
    const next = Math.max(1, Math.round((normalized / (Math.PI * 2)) * maxSeconds));
    onChange(next);
  };

  const handlePointerDown = (event: PointerEvent<SVGSVGElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    updateFromPoint(event.clientX, event.clientY);
  };

  return (
    <svg
      ref={ref}
      width="108"
      height="108"
      viewBox="0 0 108 108"
      className="shrink-0 touch-none cursor-pointer"
      onPointerDown={handlePointerDown}
      onPointerMove={(event) => {
        if (event.buttons === 1) updateFromPoint(event.clientX, event.clientY);
      }}
    >
      <circle cx="54" cy="54" r="43" fill="rgba(255,255,255,0.42)" />
      <circle
        cx="54"
        cy="54"
        r="38"
        fill="none"
        stroke="rgba(151,138,112,0.28)"
        strokeWidth="7"
      />
      <circle
        cx="54"
        cy="54"
        r="38"
        fill="none"
        stroke="currentColor"
        strokeWidth="7"
        strokeLinecap="round"
        className="text-bamboo"
        strokeDasharray={`${progress} ${circumference - progress}`}
        transform="rotate(-90 54 54)"
      />
      <line x1="54" y1="54" x2={hourHand.x} y2={hourHand.y} stroke="currentColor" strokeWidth="4" strokeLinecap="round" className="text-ink-soft/70" />
      <line x1="54" y1="54" x2={minuteHand.x} y2={minuteHand.y} stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" className="text-bamboo/80" />
      <line x1="54" y1="54" x2={secondHand.x} y2={secondHand.y} stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" className="text-red-400/70" />
      <circle cx={markerX} cy={markerY} r="6" fill="currentColor" className="text-bamboo" />
      <circle cx="54" cy="54" r="3" fill="currentColor" className="text-ink-ghost" />
    </svg>
  );
}

export function ReminderPanel({
  reminders,
  boundNote,
  onCreate,
  onDelete,
  onComplete,
  onSnooze,
  onOpenNote,
  onClose,
  compact = false,
}: ReminderPanelProps) {
  const [type, setType] = useState<ReminderType>("countdown");
  const [message, setMessage] = useState("");
  const [customSeconds, setCustomSeconds] = useState(10 * 60);
  const [alarmTime, setAlarmTime] = useState(() =>
    toLocalDateTimeValue(new Date(Date.now() + 30 * 60_000)),
  );
  const [repeat, setRepeat] = useState<ReminderRepeat>("none");
  const [bindCurrentNote, setBindCurrentNote] = useState(Boolean(boundNote));
  const [bindTouched, setBindTouched] = useState(false);
  const [ringtonePath, setRingtonePath] = useState<string | null>(() =>
    getReminderRingtonePath(),
  );
  const [error, setError] = useState<string | null>(null);

  const activeReminders = useMemo(
    () =>
      reminders
        .filter((reminder) => !reminder.done)
        .sort((left, right) => left.remindAt.localeCompare(right.remindAt)),
    [reminders],
  );

  useEffect(() => {
    if (bindTouched) return;
    setBindCurrentNote(Boolean(boundNote));
  }, [bindTouched, boundNote]);

  const createBase = () => ({
    message,
    repeat,
    noteId: bindCurrentNote ? boundNote?.id : undefined,
    noteTitle: bindCurrentNote ? boundNote?.title : undefined,
    ringtonePath: ringtonePath ?? undefined,
  });

  const handleCreateCountdown = (minutes: number) => {
    setError(null);
    onCreate(createCountdownReminder(minutes, createBase()));
    setMessage("");
  };

  const handleCreate = () => {
    setError(null);

    if (type === "countdown") {
      if (!Number.isFinite(customSeconds) || customSeconds < 1) {
        setError("倒计时至少 1 秒");
        return;
      }
      onCreate(createCountdownReminderSeconds(customSeconds, createBase()));
      setMessage("");
      return;
    }

    const remindAt = new Date(alarmTime);
    if (Number.isNaN(remindAt.getTime())) {
      setError("请选择有效的日期和时间");
      return;
    }
    if (remindAt.getTime() <= Date.now()) {
      setError("提醒时间需要晚于当前时间");
      return;
    }

    onCreate(
      createReminder({
        ...createBase(),
        type: "alarm",
        remindAt,
      }),
    );
    setMessage("");
  };

  const handleChooseRingtone = async () => {
    setError(null);
    try {
      const path = await chooseReminderRingtone();
      if (!path) return;
      setReminderRingtonePath(path);
      setRingtonePath(path);
    } catch {
      setError("选择铃声失败");
    }
  };

  return (
    <aside
      data-reminder-panel="true"
      className={`${compact ? "w-full" : "w-[340px]"} h-full shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col`}
    >
      <div className="flex items-center justify-between h-11 px-4 border-b border-paper-deep/25">
        <h2 className="text-[13px] font-display font-medium text-ink-soft">
          提醒中心
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          title="关闭提醒中心"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M2 2l8 8M10 2l-8 8" />
          </svg>
        </button>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-hidden px-4 py-4 space-y-5">
        <section className="space-y-2">
          <SlidingButtonGroup
            options={typeOptions}
            value={type}
            onChange={setType}
          />
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] text-ink-faint">提醒内容</label>
          <textarea
            value={message}
            onChange={(event) => setMessage(event.target.value)}
            placeholder={DEFAULT_REMINDER_MESSAGE}
            className="w-full min-h-[70px] px-3 py-2 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft placeholder:text-ink-ghost/60 resize-none outline-none focus:border-bamboo/35 focus:bg-cloud transition-colors"
          />
        </section>

        {type === "countdown" ? (
          <section className="space-y-2">
            <label className="block text-[11px] text-ink-faint">快捷倒计时</label>
            <div className="grid grid-cols-4 gap-1.5">
              {quickMinutes.map((minutes) => (
                <button
                  key={minutes}
                  type="button"
                  onClick={() => handleCreateCountdown(minutes)}
                  className="h-8 rounded-lg border border-paper-deep/40 bg-paper-warm/60 text-[12px] text-ink-soft hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
                >
                  {minutes}m
                </button>
              ))}
            </div>
            <div className={`${compact ? "flex flex-col items-stretch" : "flex items-center"} gap-3 rounded-xl border border-paper-deep/25 bg-paper-warm/35 p-3`}>
              <CountdownDial
                seconds={customSeconds}
                onChange={setCustomSeconds}
              />
              <div className="min-w-0 flex-1 space-y-2">
                <div className="text-[22px] font-mono text-ink tabular-nums">
                  {formatDuration(customSeconds)}
                </div>
                <div className="grid grid-cols-3 gap-1.5">
                  <TimeNumberInput
                    label="时"
                    value={Math.floor(customSeconds / 3600)}
                    max={23}
                    onChange={(hours) =>
                      setCustomSeconds(composeSeconds(hours, Math.floor((customSeconds % 3600) / 60), customSeconds % 60))
                    }
                    onIncrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds + 3600))
                    }
                    onDecrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds - 3600))
                    }
                  />
                  <TimeNumberInput
                    label="分"
                    value={Math.floor((customSeconds % 3600) / 60)}
                    max={59}
                    onChange={(minutes) =>
                      setCustomSeconds(composeSeconds(Math.floor(customSeconds / 3600), minutes, customSeconds % 60))
                    }
                    onIncrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds + 60))
                    }
                    onDecrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds - 60))
                    }
                  />
                  <TimeNumberInput
                    label="秒"
                    value={customSeconds % 60}
                    max={59}
                    onChange={(seconds) =>
                      setCustomSeconds(composeSeconds(Math.floor(customSeconds / 3600), Math.floor((customSeconds % 3600) / 60), seconds))
                    }
                    onIncrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds + 1))
                    }
                    onDecrement={() =>
                      setCustomSeconds((seconds) => clampCountdownSeconds(seconds - 1))
                    }
                  />
                </div>
              </div>
            </div>
            <div className="flex justify-end">
              <button
                type="button"
                onClick={handleCreate}
                className="h-8 px-3 rounded-lg text-[12px] text-cloud bg-bamboo hover:bg-bamboo-light transition-colors cursor-pointer"
              >
                创建
              </button>
            </div>
          </section>
        ) : (
          <section className="space-y-2">
            <label className="block text-[11px] text-ink-faint">日期和时间</label>
            <div className="flex gap-2">
              <input
                type="datetime-local"
                value={alarmTime}
                onChange={(event) => setAlarmTime(event.target.value)}
                className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none"
              />
              <button
                type="button"
                onClick={handleCreate}
                className="h-8 px-3 rounded-lg text-[12px] text-cloud bg-bamboo hover:bg-bamboo-light transition-colors cursor-pointer"
              >
                创建
              </button>
            </div>
          </section>
        )}

        <section className="space-y-2">
          <label className="block text-[11px] text-ink-faint">重复</label>
          <SlidingButtonGroup
            options={repeatOptions}
            value={repeat}
            onChange={setRepeat}
          />
        </section>

        <section className="space-y-2">
          <label className="flex items-center justify-between h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25 cursor-pointer">
            <span className="text-[12px] text-ink-soft truncate">
              {boundNote ? `绑定当前笔记：${boundNote.title}` : "没有可绑定的当前笔记"}
            </span>
            <input
              type="checkbox"
              checked={bindCurrentNote && Boolean(boundNote)}
              disabled={!boundNote}
              onChange={(event) => {
                setBindTouched(true);
                setBindCurrentNote(event.target.checked);
              }}
              className="accent-bamboo cursor-pointer disabled:cursor-not-allowed"
            />
          </label>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] text-ink-faint">铃声</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={ringtonePath ?? "默认提醒音"}
              readOnly
              className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[11px] font-mono text-ink-faint truncate"
            />
            <button
              type="button"
              onClick={handleChooseRingtone}
              className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              选择
            </button>
          </div>
          {ringtonePath && (
            <button
              type="button"
              onClick={() => {
                setReminderRingtonePath(null);
                setRingtonePath(null);
              }}
              className="text-[11px] text-ink-ghost hover:text-red-400 transition-colors cursor-pointer"
            >
              恢复默认铃声
            </button>
          )}
        </section>

        {error && <div className="text-[12px] text-red-400">{error}</div>}

        <section className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="block text-[11px] text-ink-faint">未完成提醒</label>
            <span className="text-[10px] text-ink-ghost font-mono">
              {activeReminders.length}
            </span>
          </div>
          <div className="space-y-1.5">
            {activeReminders.map((reminder) => (
              <div
                key={reminder.id}
                className="rounded-lg border border-paper-deep/30 bg-paper-warm/45 px-3 py-2"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="text-[12px] text-ink-soft truncate">
                      {reminder.message}
                    </div>
                    <div className="mt-1 text-[10px] text-ink-ghost font-mono">
                      {formatReminderTime(reminder.remindAt)} · {repeatLabel(reminder.repeat)}
                    </div>
                    {reminder.noteTitle && (
                      <div className="mt-1 text-[10px] text-ink-ghost truncate">
                        {reminder.noteTitle}
                      </div>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => onDelete(reminder.id)}
                    className="shrink-0 w-6 h-6 flex items-center justify-center rounded-md text-ink-ghost hover:text-red-400 hover:bg-danger-bg transition-colors cursor-pointer"
                    title="删除提醒"
                  >
                    <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                      <path d="M2 2l8 8M10 2l-8 8" />
                    </svg>
                  </button>
                </div>
                <div className="mt-2 flex items-center justify-end gap-1.5">
                  {reminder.noteId && onOpenNote && (
                    <button
                      type="button"
                      onClick={() => onOpenNote(reminder.noteId ?? "")}
                      className="h-7 px-2 rounded-lg border border-paper-deep/35 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
                    >
                      打开
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => onSnooze(reminder)}
                    className="h-7 px-2 rounded-lg border border-paper-deep/35 text-[11px] text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
                  >
                    稍后
                  </button>
                  <button
                    type="button"
                    onClick={() => onComplete(reminder.id)}
                    className="h-7 px-2 rounded-lg text-[11px] text-cloud bg-bamboo hover:bg-bamboo-light transition-colors cursor-pointer"
                  >
                    完成
                  </button>
                </div>
              </div>
            ))}
            {activeReminders.length === 0 && (
              <div className="py-6 text-center text-[12px] text-ink-ghost">
                暂无提醒
              </div>
            )}
          </div>
        </section>
      </div>
    </aside>
  );
}
