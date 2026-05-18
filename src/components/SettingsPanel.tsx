import { useEffect, useRef, useState } from "react";
import { useHotkeyRecorder } from "@tanstack/react-hotkeys";
import type { AppConfig, ThemeOption, TileColorMode, ViewMode } from "../features/settings/types";
import {
  formatHeldKeys,
  hotkeyToConfigString,
  isValidGlobalShortcut,
} from "../features/settings/shortcutRecorder";
import {
  DEFAULT_TILE_COLOR,
  normalizeTileColor,
} from "../features/settings/tileColor";
import { applyTheme, watchSystemTheme } from "../features/settings/theme";
import { getSystemFonts } from "../features/settings/api";
import { SlidingButtonGroup } from "./SlidingButtonGroup";

// 安全转义CSS字符串 - 纯函数，组件外部定义
function escapeCssString(str: string): string {
  return str
    .replace(/\\/g, '\\\\')  // 转义反斜杠
    .replace(/"/g, '\\"')    // 转义双引号
    .replace(/'/g, "\\'")    // 转义单引号
    .replace(/\n/g, '\\n')   // 转义换行符
    .replace(/\r/g, '\\r')   // 转义回车符
    .replace(/\t/g, '\\t');   // 转义制表符
}

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
  const [systemFonts, setSystemFonts] = useState<string[]>([]);
  
  useEffect(() => {
    // 加载系统字体
    getSystemFonts()
      .then((fonts) => {
        // 中文字体关键词
        const chineseFontKeywords = [
          "雅黑", "微软", "宋体", "黑体", "楷体", "仿宋", "新宋体",
          "宋", "黑", "楷", "仿", "体",
          "Heiti", "Songti", "Kaiti", "Fangsong",
          "PingFang", "Hiragino", "Noto",
          "WenQuanYi", "Source Han", "思源",
          "SimSun", "SimHei", "Microsoft YaHei", "KaiTi", "FangSong",
          "MingLiU", "PMingLiU", "DFHei", "STHeiti",
          "STSong", "STKaiti", "STFangsong"
        ];
        
        const isChineseFont = (fontName: string) => {
          const lowerName = fontName.toLowerCase();
          return chineseFontKeywords.some(keyword => 
            lowerName.includes(keyword.toLowerCase())
          );
        };
        
        // 将字体分为中文字体和其他字体
        const chineseFonts = fonts.filter(isChineseFont);
        const otherFonts = fonts.filter(font => !isChineseFont(font));
        
        // 分别排序
        chineseFonts.sort((a, b) => a.localeCompare(b, 'zh-CN'));
        otherFonts.sort((a, b) => a.localeCompare(b, 'en-US'));
        
        // 中文字体放在前面，然后是其他字体
        setSystemFonts([...chineseFonts, ...otherFonts]);
      })
      .catch((error) => {
        console.error("Failed to load system fonts:", error);
        // 如果加载失败，至少提供一些常用字体
        setSystemFonts([
          "Microsoft YaHei",
          "SimSun",
          "Microsoft YaHei UI",
          "SimHei",
          "KaiTi",
          "FangSong",
          "Arial",
          "Times New Roman", 
          "Courier New",
          "Segoe UI",
          "Consolas",
          "Georgia",
          "Verdana",
          "Tahoma"
        ]);
      });
  }, []);

  // 监听字体变化，更新预览字体样式和下拉选项字体
  useEffect(() => {
    // 创建或更新预览字体的样式
    let styleEl = document.getElementById('font-preview-style');
    if (!styleEl) {
      styleEl = document.createElement('style');
      styleEl.id = 'font-preview-style';
      document.head.appendChild(styleEl);
    }
    
    // 构建CSS规则：预览文本 + 下拉选项字体
    let cssRules = '';
    
    // 预览文本使用当前选中的字体
    if (config.appFont) {
      const escapedPreviewFont = escapeCssString(config.appFont);
      cssRules += `
        .font-preview-text {
          font-family: "${escapedPreviewFont}", system-ui, -apple-system, sans-serif;
        }
      `;
    }
    
    // 所有字体选项使用自己的字体显示（限制最多100个以优化性能）
    if (Array.isArray(systemFonts) && systemFonts.length > 0) {
      const fontsToProcess = systemFonts.slice(0, 100); // 限制处理数量
      fontsToProcess.forEach(font => {
        try {
          const escapedFont = escapeCssString(font);
          const escapedDataAttr = font.replace(/"/g, '\\"');
          cssRules += `
            select option[data-font="${escapedDataAttr}"] {
              font-family: "${escapedFont}", system-ui, -apple-system, sans-serif;
            }
          `;
        } catch (error) {
          // 跳过无效的字体名称
          console.warn(`Skipping invalid font: ${font}`, error);
        }
      });
    }
    
    styleEl.textContent = cssRules;
    
    return () => {
      if (styleEl) {
        styleEl.textContent = '';
      }
    };
  }, [config.appFont, systemFonts]);

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
            应用字体
          </label>
          <select
            value={config.appFont || ""}
            onChange={(event) => setConfigValue("appFont", event.target.value)}
            aria-label="应用字体选择"
            className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[11px] text-ink-soft cursor-pointer"
          >
            <option value="">系统默认</option>
            {Array.isArray(systemFonts) && systemFonts.map((font) => (
              <option key={font} value={font} data-font={font}>
                {font}
              </option>
            ))}
          </select>
          {config.appFont && (
            <div className="mt-3 p-4 rounded-lg bg-paper-warm/50 border border-paper-deep/30">
              <div className="text-[10px] text-ink-ghost mb-2">预览：</div>
              <div className="text-[16px] leading-relaxed font-preview-text">
                <div className="mb-1">滚滚长江东逝水</div>
                <div className="text-[14px]">The quick brown fox jumps over the lazy dog</div>
              </div>
            </div>
          )}
        </section>

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
              aria-label="笔记目录路径"
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
          <ToggleRow
            label="外部文件自动保存"
            checked={config.externalFileAutoSave}
            onChange={(checked) =>
              setConfigValue("externalFileAutoSave", checked)
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
              aria-label="编辑器字号"
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
              aria-label="小窗磁贴字号"
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
                aria-label="磁贴颜色选择"
                className="w-10 h-8 rounded-lg border border-paper-deep/40 bg-paper-warm/70 cursor-pointer"
              />
              <input
                type="text"
                value={config.tileColor}
                onChange={(event) =>
                  setConfigValue("tileColor", event.target.value)
                }
                placeholder="#f6f3ec"
                aria-label="磁贴颜色值"
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
        aria-label={label}
        className="sr-only"
      />
      <div
        className={`relative w-8 h-[18px] rounded-full transition-colors duration-250 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          checked ? "bg-bamboo" : "bg-paper-deep/50"
        }`}
      >
        <div
          className={`absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,0.15)] transition-transform duration-250 ease-[cubic-bezier(0.22,1,0.36,1)] ${
            checked ? "translate-x-[12px]" : "translate-x-0"
          }`}
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
  const [heldKeys, setHeldKeys] = useState<string[]>([]);
  const recorder = useHotkeyRecorder({
    onRecord: (hotkey) => {
      if (isValidGlobalShortcut(hotkey)) {
        onChange(hotkeyToConfigString(hotkey));
      }
    },
  });
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!recorder.isRecording) {
      setHeldKeys([]);
      return;
    }

    const pressed = new Set<string>();

    const toLabel = (e: KeyboardEvent): string => {
      if (e.key === "Control") return "Control";
      if (e.key === "Alt") return "Alt";
      if (e.key === "Shift") return "Shift";
      if (e.key === "Meta") return "Meta";
      return e.key.length === 1 ? e.key.toUpperCase() : e.key;
    };

    const onKeyDown = (e: KeyboardEvent) => {
      pressed.add(toLabel(e));
      setHeldKeys([...pressed]);
    };
    const onKeyUp = (e: KeyboardEvent) => {
      pressed.delete(toLabel(e));
      setHeldKeys([...pressed]);
    };
    const onBlur = () => {
      pressed.clear();
      setHeldKeys([]);
    };

    document.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("keyup", onKeyUp, true);
    window.addEventListener("blur", onBlur);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("keyup", onKeyUp, true);
      window.removeEventListener("blur", onBlur);
    };
  }, [recorder.isRecording]);

  useEffect(() => {
    if (!recorder.isRecording) return;
    const handleClick = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        recorder.cancelRecording();
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [recorder.isRecording, recorder.cancelRecording]);

  const liveDisplay =
    recorder.isRecording && heldKeys.length > 0
      ? formatHeldKeys(heldKeys)
      : null;

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => recorder.startRecording()}
        className={`w-full h-8 px-2.5 rounded-lg border text-[12px] flex items-center gap-2 cursor-pointer transition-colors ${
          recorder.isRecording
            ? "bg-bamboo-mist/40 border-bamboo"
            : "bg-paper-warm/70 border-paper-deep/40 hover:border-paper-deep/60"
        }`}
      >
        {recorder.isRecording ? (
          <>
            <span className="flex-1 text-left text-bamboo">
              {liveDisplay || "按下快捷键..."}
            </span>
            <span className="text-[10px] text-ink-faint shrink-0">
              Esc 取消
            </span>
          </>
        ) : (
          <>
            <span className="flex-1 text-left text-ink-soft">{value}</span>
            <span className="text-[10px] text-ink-ghost shrink-0">
              点击录制
            </span>
          </>
        )}
      </button>
    </div>
  );
}