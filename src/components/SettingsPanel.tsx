import { useEffect, useRef, useState } from "react";
import type { AppConfig, ThemeOption, TileColorMode, ViewMode } from "../features/settings/api";
import {
  DEFAULT_TILE_COLOR,
  normalizeTileColor,
} from "../features/settings/tileColor";
import { applyTheme, watchSystemTheme } from "../features/settings/theme";
import { SlidingButtonGroup } from "./SlidingButtonGroup";

const tileColorModes: Array<{ value: TileColorMode; label: string }> = [
  { value: "system", label: "跟随主题" },
  { value: "custom", label: "自定义" },
];

interface SettingsPanelProps {
  config: AppConfig;
  onChange: (config: AppConfig) => void;
  onChooseNotesDir: () => void;
  onClose: () => void;
}

const themeOptions: Array<{ value: ThemeOption; label: string }> = [
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "跟随系统" },
];

const viewModes: Array<{ value: ViewMode; label: string }> = [
  { value: "edit", label: "编辑" },
  { value: "split", label: "分栏" },
  { value: "preview", label: "预览" },
];

export function SettingsPanel({
  config,
  onChange,
  onChooseNotesDir,
  onClose,
}: SettingsPanelProps) {
  const setConfigValue = <Key extends keyof AppConfig>(
    key: Key,
    value: AppConfig[Key],
  ) => {
    onChange({ ...config, [key]: value });
  };

  return (
    <aside className="w-[360px] h-full shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col">
      <div className="flex items-center justify-between h-11 px-4 border-b border-paper-deep/25">
        <h2 className="text-[13px] font-display font-medium text-ink-soft">
          应用设置
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          title="关闭设置"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
          >
            <path d="M2 2l8 8M10 2l-8 8" />
          </svg>
        </button>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-hidden px-4 py-4 space-y-5">
        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            主题
          </label>
          <SlidingButtonGroup
            options={themeOptions}
            value={config.theme}
            onChange={(v: ThemeOption) => {
              setConfigValue("theme", v);
              applyTheme(v);
              watchSystemTheme(v);
            }}
          />
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            笔记目录
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={config.notesDir}
              readOnly
              className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[11px] font-mono text-ink-faint truncate"
            />
            <button
              type="button"
              onClick={onChooseNotesDir}
              className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              选择文件夹
            </button>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            快捷键
          </label>
          <ShortcutRecorder
            value={config.globalShortcut}
            onChange={(v) => setConfigValue("globalShortcut", v)}
          />
        </section>

        <section className="space-y-2">
          <ToggleRow
            label="关闭到托盘"
            checked={config.closeToTray}
            onChange={(checked) => setConfigValue("closeToTray", checked)}
          />
          <ToggleRow
            label="开机自启"
            checked={config.autostart}
            onChange={(checked) => setConfigValue("autostart", checked)}
          />
          <ToggleRow
            label="自动保存笔记"
            checked={config.noteAutoSave}
            onChange={(checked) => setConfigValue("noteAutoSave", checked)}
          />
          <ToggleRow
            label="小窗笔记自动保存"
            checked={config.noteSurfaceAutoSave}
            onChange={(checked) =>
              setConfigValue("noteSurfaceAutoSave", checked)
            }
          />
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            编辑器字号
          </label>
          <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
            <input
              type="range"
              min={8}
              max={30}
              step={1}
              value={config.fontSize ?? 14}
              onChange={(event) =>
                setConfigValue("fontSize", Number(event.target.value))
              }
              className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
            />
            <span className="text-[12px] font-mono text-ink-soft tabular-nums w-8 text-right">
              {config.fontSize ?? 14}px
            </span>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            小窗/磁贴字号
          </label>
          <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
            <input
              type="range"
              min={8}
              max={30}
              step={1}
              value={config.surfaceFontSize ?? 14}
              onChange={(event) =>
                setConfigValue("surfaceFontSize", Number(event.target.value))
              }
              className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
            />
            <span className="text-[12px] font-mono text-ink-soft tabular-nums w-8 text-right">
              {config.surfaceFontSize ?? 14}px
            </span>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            磁贴颜色
          </label>
          <SlidingButtonGroup
            options={tileColorModes}
            value={config.tileColorMode}
            onChange={(v: TileColorMode) => setConfigValue("tileColorMode", v)}
          />
          {config.tileColorMode === "custom" && (
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={normalizeTileColor(config.tileColor)}
                onChange={(event) =>
                  setConfigValue("tileColor", event.target.value)
                }
                className="w-10 h-8 rounded-lg border border-paper-deep/40 bg-paper-warm/70 cursor-pointer"
              />
              <input
                type="text"
                value={config.tileColor}
                onChange={(event) =>
                  setConfigValue("tileColor", event.target.value)
                }
                placeholder="#f6f3ec"
                spellCheck={false}
                className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] font-mono text-ink-soft outline-none"
              />
              <button
                type="button"
                onClick={() => setConfigValue("tileColor", DEFAULT_TILE_COLOR)}
                className="h-8 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer whitespace-nowrap"
              >
                默认
              </button>
            </div>
          )}
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            默认视图
          </label>
          <SlidingButtonGroup
            options={viewModes}
            value={config.defaultViewMode}
            onChange={(v) => setConfigValue("defaultViewMode", v)}
          />
        </section>
      </div>

    </aside>
  );
}

interface ToggleRowProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

function ToggleRow({ label, checked, onChange }: ToggleRowProps) {
  return (
    <label className="flex items-center justify-between h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25 cursor-pointer">
      <span className="text-[12px] text-ink-soft">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="sr-only"
      />
      <div
        className={`relative w-8 h-[18px] rounded-full transition-colors duration-250 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          checked ? "bg-bamboo" : "bg-paper-deep/50"
        }`}
      >
        <div
          className="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,0.15)]"
          style={{
            transform: `translateX(${checked ? 14 : 0}px)`,
            transition: "transform 250ms cubic-bezier(0.22, 1, 0.36, 1)",
            willChange: "transform",
          }}
        />
      </div>
    </label>
  );
}

interface ShortcutRecorderProps {
  value: string;
  onChange: (value: string) => void;
}

function ShortcutRecorder({ value, onChange }: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!recording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecording(false);
        return;
      }

      const modifiers: string[] = [];
      if (e.ctrlKey || e.metaKey) modifiers.push("Ctrl");
      if (e.altKey) modifiers.push("Alt");
      if (e.shiftKey) modifiers.push("Shift");
      if (e.metaKey && !e.ctrlKey) modifiers.push("Super");

      const key = e.key;
      if (key === "Control" || key === "Alt" || key === "Shift" || key === "Meta") {
        return;
      }

      let keyName = key
        .replace(/^Arrow/, "")
        .replace(/^Digit/, "")
        .replace(/^Key/, "")
        .toUpperCase();

      if (key.length === 1) {
        keyName = key.toUpperCase();
      }

      if (["Space", "Tab", "Enter", "Escape", "Backspace", "Delete"].includes(key)) {
        keyName = key;
      }

      if (key.startsWith("F") && !isNaN(Number(key.slice(1)))) {
        keyName = key;
      }

      if (modifiers.length === 0) {
        return;
      }

      const shortcut = [...modifiers, keyName].join("+");
      onChange(shortcut);
      setRecording(false);
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [recording, onChange]);

  useEffect(() => {
    if (!recording) return;
    const handleClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setRecording(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [recording]);

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => setRecording(true)}
        className={`w-full h-8 px-2.5 rounded-lg border text-[12px] flex items-center justify-between cursor-pointer transition-colors ${
          recording
            ? "bg-bamboo-mist/40 border-bamboo text-bamboo"
            : "bg-paper-warm/70 border-paper-deep/40 text-ink-soft hover:border-paper-deep/60"
        }`}
      >
        <span>{recording ? "按下快捷键..." : value}</span>
        {recording ? (
          <span className="text-[10px] text-ink-faint">按 Esc 取消</span>
        ) : (
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-ink-ghost"
          >
            <path d="M2 3.5l3 3 3-3" />
          </svg>
        )}
      </button>
    </div>
  );
}
