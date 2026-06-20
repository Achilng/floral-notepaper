import { memo, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { OutlineItem } from "../features/markdown/outline";

interface OutlinePanelProps {
  items: OutlineItem[];
  /** Slug of the heading to mark as active (currently in view), if any. */
  activeSlug?: string | null;
  onSelect: (item: OutlineItem) => void;
}

function OutlinePanelImpl({ items, activeSlug, onSelect }: OutlinePanelProps) {
  const { t } = useTranslation();
  const activeRef = useRef<HTMLButtonElement>(null);

  // Keep the followed heading visible as the cursor/scroll moves through the doc.
  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [activeSlug]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center min-h-10 px-4 border-b border-paper-deep/20 shrink-0">
        <span className="text-[11px] font-medium text-ink-faint tracking-wide">
          {t("main.outline.title", { defaultValue: "大纲" })}
        </span>
      </div>

      {items.length === 0 ? (
        <div className="flex-1 flex items-center justify-center px-4 text-center text-[11px] text-ink-ghost/70">
          {t("main.outline.empty", { defaultValue: "暂无标题" })}
        </div>
      ) : (
        <nav className="flex-1 overflow-y-auto py-2">
          {items.map((item) => {
            const active = activeSlug != null && item.slug === activeSlug;
            return (
              <button
                key={item.slug}
                ref={active ? activeRef : undefined}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onSelect(item)}
                title={item.text}
                className={`w-full text-left truncate py-1 pr-3 text-[12px] font-body transition-colors cursor-pointer border-l-2 ${
                  active
                    ? "border-bamboo text-bamboo bg-bamboo-mist/40"
                    : "border-transparent text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/30"
                }`}
                style={{ paddingLeft: `${0.75 + (item.level - 1) * 0.85}rem` }}
              >
                {item.text}
              </button>
            );
          })}
        </nav>
      )}
    </div>
  );
}

// memo：items 在大纲关闭时是稳定空数组、onSelect 已 useCallback、其余为基本类型，
// 浅比较使“关闭大纲时按键”或与大纲无关的状态变更不再重渲整列。
export const OutlinePanel = memo(OutlinePanelImpl);
