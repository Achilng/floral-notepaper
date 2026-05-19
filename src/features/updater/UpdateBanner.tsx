import type { UpdateInfo } from "./api";

interface UpdateBannerProps {
  update: UpdateInfo;
  onDownload: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({ update, onDownload, onDismiss }: UpdateBannerProps) {
  return (
    <div className="fixed bottom-4 right-4 z-50 animate-fade-up">
      <div className="w-[320px] rounded-xl bg-cloud border border-paper-deep/40 shadow-lg shadow-shadow-deep p-4">
        <div className="flex items-start justify-between gap-3 mb-2">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-bamboo animate-pulse" />
            <span className="text-[13px] font-display font-medium text-ink">
              发现新版本 v{update.version}
            </span>
          </div>
          <button
            type="button"
            onClick={onDismiss}
            className="w-6 h-6 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer shrink-0"
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M2 2l8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>

        {update.body && (
          <p className="text-[11px] text-ink-faint leading-relaxed mb-3 max-h-[80px] overflow-y-auto scrollbar-hidden">
            {update.body.length > 200 ? update.body.slice(0, 200) + "…" : update.body}
          </p>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            onClick={onDownload}
            className="flex-1 h-8 rounded-lg bg-bamboo text-white text-[12px] font-body hover:bg-bamboo-light transition-colors cursor-pointer"
          >
            前往下载
          </button>
          <button
            type="button"
            onClick={onDismiss}
            className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          >
            稍后
          </button>
        </div>
      </div>
    </div>
  );
}
