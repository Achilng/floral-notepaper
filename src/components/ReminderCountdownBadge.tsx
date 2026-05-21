interface ReminderCountdownBadgeProps {
  value: string;
  className?: string;
}

export function ReminderCountdownBadge({
  value,
  className = "",
}: ReminderCountdownBadgeProps) {
  return (
    <div
      className={`inline-flex items-center gap-1.5 h-7 px-2.5 rounded-lg bg-bamboo-mist/65 border border-bamboo/15 text-bamboo shadow-[0_1px_6px_rgba(26,26,24,0.06)] ${className}`}
      title="绑定提醒倒计时"
    >
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M6 8a6 6 0 0 1 12 0c0 7 3 7 3 9H3c0-2 3-2 3-9" />
        <path d="M10 21h4" />
      </svg>
      <span className="text-[11px] font-mono tabular-nums leading-none">
        {value}
      </span>
    </div>
  );
}
