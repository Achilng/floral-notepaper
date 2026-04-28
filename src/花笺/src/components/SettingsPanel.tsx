import type { AppConfig, ViewMode } from "../features/settings/types";
import { supportedShortcuts } from "../features/settings/api";

interface SettingsPanelProps {
  config: AppConfig;
  isSaving: boolean;
  onChange: (config: AppConfig) => void;
  onChooseNotesDir: () => void;
  onClose: () => void;
  onSave: () => void;
}

const viewModes: Array<{ value: ViewMode; label: string }> = [
  { value: "edit", label: "编辑" },
  { value: "split", label: "分栏" },
  { value: "preview", label: "预览" },
];

export function SettingsPanel({
  config,
  isSaving,
  onChange,
  onChooseNotesDir,
  onClose,
  onSave,
}: SettingsPanelProps) {
  const setConfigValue = <Key extends keyof AppConfig>(
    key: Key,
    value: AppConfig[Key],
  ) => {
    onChange({ ...config, [key]: value });
  };

  return (
    <aside className="w-[360px] shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col">
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

      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-5">
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
          <select
            value={config.globalShortcut}
            onChange={(event) =>
              setConfigValue("globalShortcut", event.target.value)
            }
            className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none"
          >
            {supportedShortcuts.map((shortcut) => (
              <option key={shortcut} value={shortcut}>
                {shortcut}
              </option>
            ))}
          </select>
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
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            默认视图
          </label>
          <div className="grid grid-cols-3 gap-1 bg-paper-warm/60 rounded-lg p-[2px] border border-paper-deep/30">
            {viewModes.map((mode) => (
              <button
                key={mode.value}
                type="button"
                onClick={() => setConfigValue("defaultViewMode", mode.value)}
                className={`h-7 rounded-md text-[11px] transition-all cursor-pointer ${
                  config.defaultViewMode === mode.value
                    ? "bg-cloud text-bamboo font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                    : "text-ink-ghost hover:text-ink-faint"
                }`}
              >
                {mode.label}
              </button>
            ))}
          </div>
        </section>
      </div>

      <div className="px-4 py-3 border-t border-paper-deep/25 bg-paper/25">
        <button
          type="button"
          onClick={onSave}
          disabled={isSaving}
          className="w-full h-8 rounded-lg bg-bamboo text-cloud text-[12px] font-body hover:bg-bamboo-light transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? "保存中" : "保存设置"}
        </button>
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
    <label className="flex items-center justify-between h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
      <span className="text-[12px] text-ink-soft">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="h-4 w-4 accent-bamboo"
      />
    </label>
  );
}
