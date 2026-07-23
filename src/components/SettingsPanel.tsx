import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  checkGlobalShortcut,
  chooseBackgroundImage,
  loadSystemFonts,
} from "../features/settings/api";
import { UpdateSettingsSection } from "../features/update/UpdateSettingsSection";
import type {
  AppConfig,
  BackgroundFit,
  ThemeOption,
  TileColorMode,
  ViewMode,
} from "../features/settings/types";
import {
  formatHeldKeys,
  hotkeyToConfigString,
  isValidGlobalShortcut,
  shortcutPlatform,
} from "../features/settings/shortcutRecorder";
import { useShortcutRecorder } from "../features/settings/useShortcutRecorder";
import { DEFAULT_TILE_COLOR, normalizeTileColor } from "../features/settings/tileColor";
import { applyTheme, watchSystemTheme } from "../features/settings/theme";
import { LOCALE_OPTIONS } from "../locales/locale-whitelist";
import { SlidingButtonGroup } from "./SlidingButtonGroup";

const HARMONY_FONT_LICENSE_URL = new URL("../assets/fonts/LICENSE_Fonts", import.meta.url).href;
const DEFAULT_INTERFACE_BACKGROUND = "#f6f3ec";

type SettingsView = "main" | "interface";

const FONT_OPTIONS_CACHE = {
  loaded: false,
  loading: null as Promise<string[]> | null,
  values: [] as string[],
};

function loadCachedSystemFonts(): Promise<string[]> {
  if (FONT_OPTIONS_CACHE.loaded) return Promise.resolve(FONT_OPTIONS_CACHE.values);
  if (!FONT_OPTIONS_CACHE.loading) {
    FONT_OPTIONS_CACHE.loading = loadSystemFonts()
      .then((fonts) => {
        FONT_OPTIONS_CACHE.values = fonts;
        FONT_OPTIONS_CACHE.loaded = true;
        return fonts;
      })
      .catch(() => {
        FONT_OPTIONS_CACHE.values = [];
        FONT_OPTIONS_CACHE.loaded = true;
        return [];
      })
      .finally(() => {
        FONT_OPTIONS_CACHE.loading = null;
      });
  }
  return FONT_OPTIONS_CACHE.loading;
}

interface SettingsPanelProps {
  config: AppConfig;
  onChange: (config: AppConfig) => void;
  onMigrateDataDir: () => void;
  onClose: () => void;
}

export function SettingsPanel({ config, onChange, onMigrateDataDir, onClose }: SettingsPanelProps) {
  const { t } = useTranslation();
  const [settingsView, setSettingsView] = useState<SettingsView>("main");
  const [fontOptions, setFontOptions] = useState<string[]>(FONT_OPTIONS_CACHE.values);
  const [fontOptionsRequested, setFontOptionsRequested] = useState(FONT_OPTIONS_CACHE.loaded);
  const setConfigValue = <Key extends keyof AppConfig>(key: Key, value: AppConfig[Key]) => {
    onChange({ ...config, [key]: value });
  };

  useEffect(() => {
    if (!fontOptionsRequested) return;
    let cancelled = false;
    void loadCachedSystemFonts().then((fonts) => {
      if (!cancelled) setFontOptions(fonts);
    });
    return () => {
      cancelled = true;
    };
  }, [fontOptionsRequested]);
  const tileColorModes = useMemo<Array<{ value: TileColorMode; label: string }>>(
    () => [
      {
        value: "system",
        label: t("settings.tileColor.followTheme", { defaultValue: "跟随主题" }),
      },
      {
        value: "custom",
        label: t("settings.tileColor.custom", { defaultValue: "自定义" }),
      },
    ],
    [t],
  );
  const themeOptions = useMemo<Array<{ value: ThemeOption; label: string }>>(
    () => [
      { value: "light", label: t("settings.theme.light", { defaultValue: "浅色" }) },
      { value: "dark", label: t("settings.theme.dark", { defaultValue: "深色" }) },
      {
        value: "system",
        label: t("settings.theme.system", { defaultValue: "跟随系统" }),
      },
    ],
    [t],
  );
  const viewModes = useMemo<Array<{ value: ViewMode; label: string }>>(
    () => [
      { value: "edit", label: t("settings.defaultView.edit", { defaultValue: "编辑" }) },
      { value: "split", label: t("settings.defaultView.split", { defaultValue: "分栏" }) },
      {
        value: "preview",
        label: t("settings.defaultView.preview", { defaultValue: "预览" }),
      },
    ],
    [t],
  );
  const localeOptions = useMemo(
    () =>
      LOCALE_OPTIONS.map(({ value, labelKey, defaultLabel }) => ({
        value,
        label: t(labelKey, { defaultValue: defaultLabel }),
      })),
    [t],
  );

  if (settingsView === "interface") {
    return (
      <aside className="w-[360px] h-full shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col">
        <div className="flex items-center justify-between h-11 px-4 border-b border-paper-deep/25">
          <button
            type="button"
            onClick={() => setSettingsView("main")}
            className="h-7 px-2 -ml-1 flex items-center gap-1.5 rounded-lg text-[12px] text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          >
            <span aria-hidden="true">‹</span>
            {t("settings.back", { defaultValue: "返回" })}
          </button>
          <h2 className="text-[13px] font-display font-medium text-ink-soft">
            {t("settings.interface.title", { defaultValue: "界面设置" })}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
            title={t("settings.closeTitle", { defaultValue: "关闭设置" })}
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
        <InterfaceSettingsPage
          config={config}
          fontOptions={fontOptions}
          onRequestFontOptions={() => setFontOptionsRequested(true)}
          onPatch={(patch) => onChange({ ...config, ...patch })}
        />
      </aside>
    );
  }

  return (
    <aside className="w-[360px] h-full shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col">
      <div className="flex items-center justify-between h-11 px-4 border-b border-paper-deep/25">
        <h2 className="text-[13px] font-display font-medium text-ink-soft">
          {t("settings.title", { defaultValue: "应用设置" })}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          title={t("settings.closeTitle", { defaultValue: "关闭设置" })}
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
            {t("settings.theme.label", { defaultValue: "主题" })}
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
            {t("settings.dataDir", { defaultValue: "数据目录" })}
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={config.dataDir}
              readOnly
              className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[11px] font-mono text-ink-faint truncate"
            />
            <button
              type="button"
              onClick={onMigrateDataDir}
              className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              {t("settings.selectFolder", { defaultValue: "选择文件夹" })}
            </button>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            {t("settings.locale.label", { defaultValue: "语言" })}
          </label>
          <SlidingButtonGroup
            options={localeOptions}
            value={config.locale}
            onChange={(value) => setConfigValue("locale", value)}
          />
        </section>

        <section className="space-y-2">
          <NavigationRow
            label={t("settings.interface.entry", { defaultValue: "界面设置" })}
            description={t("settings.interface.entryHint", {
              defaultValue: "字体、字号和背景颜色",
            })}
            onClick={() => setSettingsView("interface")}
          />
        </section>

        <section className="space-y-2">
          <ToggleRow
            label={t("settings.closeToTray", { defaultValue: "关闭到托盘" })}
            checked={config.closeToTray}
            onChange={(checked) => setConfigValue("closeToTray", checked)}
          />
          <ToggleRow
            label={t("settings.autostart", { defaultValue: "开机自启" })}
            checked={config.autostart}
            onChange={(checked) => setConfigValue("autostart", checked)}
          />
          <ToggleRow
            label={t("settings.autoSave.note", { defaultValue: "自动保存笔记" })}
            checked={config.noteAutoSave}
            onChange={(checked) => setConfigValue("noteAutoSave", checked)}
          />
          <ToggleRow
            label={t("settings.autoSave.surface", { defaultValue: "小窗笔记自动保存" })}
            checked={config.noteSurfaceAutoSave}
            onChange={(checked) => setConfigValue("noteSurfaceAutoSave", checked)}
          />
          <ToggleRow
            label={t("settings.autoSave.externalFile", { defaultValue: "外部文件自动保存" })}
            checked={config.externalFileAutoSave}
            onChange={(checked) => setConfigValue("externalFileAutoSave", checked)}
          />
          <ToggleRow
            label={t("settings.rememberSurfaceSize", { defaultValue: "记住小窗尺寸" })}
            checked={config.rememberSurfaceSize}
            onChange={(checked) => setConfigValue("rememberSurfaceSize", checked)}
          />
          <ToggleRow
            label={t("settings.rememberWindowBounds", { defaultValue: "记住窗口大小和位置" })}
            checked={config.rememberWindowBounds ?? true}
            onChange={(checked) => setConfigValue("rememberWindowBounds", checked)}
          />
          <ToggleRow
            label={t("settings.tileRenderMarkdown", { defaultValue: "磁贴渲染 Markdown" })}
            checked={config.tileRenderMarkdown}
            onChange={(checked) => setConfigValue("tileRenderMarkdown", checked)}
          />
          <ToggleRow
            label={t("settings.tileDoubleClickToEdit", { defaultValue: "双击磁贴进入编辑" })}
            checked={config.tileDoubleClickToEdit ?? false}
            onChange={(checked) => setConfigValue("tileDoubleClickToEdit", checked)}
          />
          <ToggleRow
            label={t("settings.tileSaveReturnsToPin", { defaultValue: "保存后回到磁贴" })}
            checked={config.tileSaveReturnsToPin ?? false}
            onChange={(checked) => setConfigValue("tileSaveReturnsToPin", checked)}
          />
          <ToggleRow
            label={t("settings.renderHtmlMarkdown", { defaultValue: "允许 HTML 标签渲染" })}
            checked={config.renderHtmlMarkdown}
            onChange={(checked) => setConfigValue("renderHtmlMarkdown", checked)}
          />
          <ToggleRow
            label={t("settings.splitScrollSync", { defaultValue: "分栏同步滚动" })}
            checked={config.splitScrollSync ?? true}
            onChange={(checked) => setConfigValue("splitScrollSync", checked)}
          />
        </section>

        {/* 快捷键功能设置区域，与上方常规设置分开 */}
        <section className="space-y-2">
          <ToggleRow
            label={t("settings.tileCtrlClose", { defaultValue: "Ctrl+右键快速关闭磁贴" })}
            checked={config.tileCtrlClose}
            onChange={(checked) => setConfigValue("tileCtrlClose", checked)}
          />
          <ToggleRow
            label={t("settings.openAtCursor", { defaultValue: "快捷键打开时跟随鼠标位置" })}
            checked={config.openAtCursor ?? true}
            onChange={(checked) => setConfigValue("openAtCursor", checked)}
          />
          <div className="space-y-1.5">
            <label className="block text-[11px] font-body text-ink-faint/70 px-0.5">
              {t("settings.quickNoteShortcut", { defaultValue: "快捷记录快捷键" })}
            </label>
            <ShortcutRecorder
              value={config.globalShortcut}
              onChange={(v) => setConfigValue("globalShortcut", v)}
            />
          </div>
          <div className="space-y-1.5">
            <label className="block text-[11px] font-body text-ink-faint/70 px-0.5">
              {t("settings.visibilityShortcut", { defaultValue: "显示/隐藏窗口快捷键" })}
            </label>
            <ShortcutRecorder
              value={config.toggleVisibilityShortcut}
              onChange={(v) => setConfigValue("toggleVisibilityShortcut", v)}
            />
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            {t("settings.tabIndentSize", { defaultValue: "Tab 缩进宽��" })}
          </label>
          <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
            <input
              type="range"
              min={1}
              max={8}
              step={1}
              value={config.tabIndentSize ?? 2}
              onChange={(event) => setConfigValue("tabIndentSize", Number(event.target.value))}
              className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
            />
            <span className="text-[12px] font-mono text-ink-soft tabular-nums w-10 text-right">
              {config.tabIndentSize ?? 2}
            </span>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            {t("settings.tileColor.label", { defaultValue: "磁贴颜色" })}
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
                onChange={(event) => setConfigValue("tileColor", event.target.value)}
                className="w-10 h-8 rounded-lg border border-paper-deep/40 bg-paper-warm/70 cursor-pointer"
              />
              <input
                type="text"
                value={config.tileColor}
                onChange={(event) => setConfigValue("tileColor", event.target.value)}
                placeholder="#f6f3ec"
                spellCheck={false}
                className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] font-mono text-ink-soft outline-none"
              />
              <button
                type="button"
                onClick={() => setConfigValue("tileColor", DEFAULT_TILE_COLOR)}
                className="h-8 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer whitespace-nowrap"
              >
                {t("common.default", { defaultValue: "默认" })}
              </button>
            </div>
          )}
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            {t("settings.defaultView.label", { defaultValue: "默认视图" })}
          </label>
          <SlidingButtonGroup
            options={viewModes}
            value={config.defaultViewMode}
            onChange={(v) => setConfigValue("defaultViewMode", v)}
          />
        </section>

        <UpdateSettingsSection mode="settingsOnly" />

        <section className="pt-2 border-t border-paper-deep/25">
          <p className="text-[10px] leading-relaxed text-ink-ghost/75">
            <span>
              {t("settings.fontNotice", {
                defaultValue:
                  "Uses HarmonyOS Sans SC font. Copyright 2021 Huawei Device Co., Ltd. Licensed under HarmonyOS Sans Fonts License Agreement.",
              })}
            </span>{" "}
            <a
              href={HARMONY_FONT_LICENSE_URL}
              target="_blank"
              rel="noreferrer"
              className="underline underline-offset-2 hover:text-ink-faint"
            >
              HarmonyOS Sans Fonts License Agreement
            </a>
          </p>
        </section>
      </div>
    </aside>
  );
}

interface NavigationRowProps {
  label: string;
  description?: string;
  onClick: () => void;
}

function NavigationRow({ label, description, onClick }: NavigationRowProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full flex items-center justify-between min-h-11 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25 hover:bg-bamboo-mist/45 transition-colors cursor-pointer text-left"
    >
      <span className="min-w-0">
        <span className="block text-[12px] text-ink-soft">{label}</span>
        {description && (
          <span className="block text-[10px] text-ink-ghost mt-0.5">{description}</span>
        )}
      </span>
      <span className="text-[16px] text-ink-ghost ml-2">›</span>
    </button>
  );
}

interface InterfaceSettingsPageProps {
  config: AppConfig;
  fontOptions: string[];
  onRequestFontOptions: () => void;
  onPatch: (patch: Partial<AppConfig>) => void;
}

function InterfaceSettingsPage({
  config,
  fontOptions,
  onRequestFontOptions,
  onPatch,
}: InterfaceSettingsPageProps) {
  const { t } = useTranslation();
  const selectedFont = config.interfaceFontFamily || "HarmonyOS Sans SC";
  const backgroundColor = normalizeHexColor(config.backgroundColor, DEFAULT_INTERFACE_BACKGROUND);

  return (
    <div className="flex-1 overflow-y-auto scrollbar-hidden px-4 py-4 space-y-5">
      <section className="space-y-2">
        <label className="block text-[11px] font-body text-ink-faint">
          {t("settings.interface.fontFamily", { defaultValue: "界面字体" })}
        </label>
        <FontSelect
          value={selectedFont}
          options={fontOptions}
          onOpen={onRequestFontOptions}
          onChange={(font) => onPatch({ interfaceFontFamily: font })}
        />
      </section>

      <section className="space-y-2">
        <label className="block text-[11px] font-body text-ink-faint">
          {t("settings.fontSize.editor", { defaultValue: "编辑器字号" })}
        </label>
        <RangeRow
          label="Aa"
          value={config.fontSize ?? 14}
          min={8}
          max={30}
          step={1}
          format={(value) => `${value}px`}
          onChange={(value) => onPatch({ fontSize: value })}
        />
      </section>

      <section className="space-y-2">
        <label className="block text-[11px] font-body text-ink-faint">
          {t("settings.fontSize.surface", { defaultValue: "小窗/磁贴字号" })}
        </label>
        <RangeRow
          label="Aa"
          value={config.surfaceFontSize ?? 14}
          min={8}
          max={30}
          step={1}
          format={(value) => `${value}px`}
          onChange={(value) => onPatch({ surfaceFontSize: value })}
        />
      </section>

      <section className="space-y-2">
        <label className="block text-[11px] font-body text-ink-faint">
          {t("settings.interface.backgroundColor", { defaultValue: "背景颜色" })}
        </label>
        <ColorPalette
          value={backgroundColor}
          onChange={(color) => onPatch({ backgroundColor: color })}
          onReset={() => onPatch({ backgroundColor: "" })}
        />
      </section>

      <section className="space-y-2 pt-2 border-t border-paper-deep/25">
        <label className="block text-[11px] font-body text-ink-faint">
          {t("settings.background.label", { defaultValue: "背景图片" })}
        </label>
        <BackgroundImageSettings config={config} onPatch={onPatch} />
      </section>
    </div>
  );
}

interface FontSelectProps {
  value: string;
  options: string[];
  onOpen: () => void;
  onChange: (value: string) => void;
}

function FontSelect({ value, options, onOpen, onChange }: FontSelectProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const normalizedOptions = useMemo(() => {
    const fonts = ["HarmonyOS Sans SC", ...options].filter(Boolean);
    return Array.from(new Set(fonts));
  }, [options]);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => {
          setOpen((current) => !current);
          onOpen();
        }}
        className="w-full h-9 px-2.5 rounded-lg bg-paper-warm/45 border border-paper-deep/25 flex items-center justify-between text-left cursor-pointer hover:bg-paper-warm/70 transition-colors"
      >
        <span className="min-w-0 truncate text-[12px] text-ink-soft" style={{ fontFamily: value }}>
          {value || t("settings.interface.defaultFont", { defaultValue: "默认字体" })}
        </span>
        <span
          className={`text-[12px] text-ink-ghost transition-transform duration-200 ${open ? "rotate-180" : ""}`}
        >
          ⌄
        </span>
      </button>
      <div
        className={`overflow-hidden transition-[max-height,opacity,transform] duration-220 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          open ? "max-h-56 opacity-100 translate-y-0" : "max-h-0 opacity-0 -translate-y-1"
        }`}
      >
        <div className="mt-2 max-h-52 overflow-y-auto rounded-lg border border-paper-deep/35 bg-cloud/95 shadow-[0_8px_24px_rgba(26,26,24,0.08)] p-1 scrollbar-hidden">
          {normalizedOptions.map((font) => (
            <button
              key={font}
              type="button"
              onClick={() => {
                onChange(font);
                setOpen(false);
              }}
              className={`w-full h-8 px-2 rounded-md flex items-center justify-between text-left text-[12px] transition-colors cursor-pointer ${
                font === value
                  ? "bg-bamboo-mist text-bamboo"
                  : "text-ink-soft hover:bg-paper-warm/70"
              }`}
              style={{ fontFamily: font }}
            >
              <span className="truncate">{font}</span>
              {font === value && <span className="text-[10px]">✓</span>}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

interface ColorPaletteProps {
  value: string;
  onChange: (value: string) => void;
  onReset: () => void;
}

function ColorPalette({ value, onChange, onReset }: ColorPaletteProps) {
  const { t } = useTranslation();
  const [hsv, setHsv] = useState(() => hexToHsv(value));
  const paletteRef = useRef<HTMLDivElement>(null);
  const hueRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setHsv(hexToHsv(value));
  }, [value]);

  const commit = (next: HsvColor) => {
    setHsv(next);
    onChange(hsvToHex(next));
  };

  const updateFromPointer = (event: PointerEvent<HTMLDivElement>, target: "palette" | "hue") => {
    const ref = target === "palette" ? paletteRef : hueRef;
    const rect = ref.current?.getBoundingClientRect();
    if (!rect) return;
    const x = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    const y = clamp((event.clientY - rect.top) / rect.height, 0, 1);
    if (target === "palette") {
      commit({ ...hsv, s: x, v: 1 - y });
    } else {
      commit({ ...hsv, h: y });
    }
  };

  return (
    <div className="rounded-xl border border-paper-deep/25 bg-paper-warm/35 p-3 space-y-3">
      <div className="flex items-center gap-2">
        <div
          className="w-9 h-9 rounded-lg border border-paper-deep/45 shadow-inner"
          style={{ backgroundColor: value }}
        />
        <input
          type="text"
          value={value}
          onChange={(event) => onChange(normalizeHexColor(event.target.value, value))}
          spellCheck={false}
          className="min-w-0 flex-1 h-9 px-2.5 rounded-lg bg-cloud/70 border border-paper-deep/35 text-[12px] font-mono text-ink-soft outline-none"
        />
        <button
          type="button"
          onClick={onReset}
          className="h-9 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
        >
          {t("common.default", { defaultValue: "默认" })}
        </button>
      </div>
      <div className="flex gap-3">
        <div
          ref={paletteRef}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            updateFromPointer(event, "palette");
          }}
          onPointerMove={(event) => {
            if (event.currentTarget.hasPointerCapture(event.pointerId))
              updateFromPointer(event, "palette");
          }}
          className="relative h-28 flex-1 rounded-lg overflow-hidden border border-paper-deep/35 cursor-crosshair"
          style={{
            background: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${Math.round(
              hsv.h * 360,
            )} 100% 50%))`,
          }}
        >
          <span
            className="absolute w-3 h-3 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.35),0_2px_4px_rgba(0,0,0,0.2)] -translate-x-1/2 -translate-y-1/2 pointer-events-none"
            style={{ left: `${hsv.s * 100}%`, top: `${(1 - hsv.v) * 100}%` }}
          />
        </div>
        <div
          ref={hueRef}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            updateFromPointer(event, "hue");
          }}
          onPointerMove={(event) => {
            if (event.currentTarget.hasPointerCapture(event.pointerId))
              updateFromPointer(event, "hue");
          }}
          className="relative w-4 h-28 rounded-full cursor-pointer border border-paper-deep/35"
          style={{
            background: "linear-gradient(to bottom, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)",
          }}
        >
          <span
            className="absolute left-1/2 w-5 h-2 rounded-full bg-white border border-ink-ghost shadow -translate-x-1/2 -translate-y-1/2 pointer-events-none"
            style={{ top: `${hsv.h * 100}%` }}
          />
        </div>
      </div>
    </div>
  );
}

function BackgroundImageSettings({
  config,
  onPatch,
}: {
  config: AppConfig;
  onPatch: (patch: Partial<AppConfig>) => void;
}) {
  const { t } = useTranslation();
  const backgroundFits = useMemo<Array<{ value: BackgroundFit; label: string }>>(
    () => [
      { value: "cover", label: t("settings.background.fit.cover", { defaultValue: "填充" }) },
      { value: "contain", label: t("settings.background.fit.contain", { defaultValue: "完整" }) },
      { value: "repeat", label: t("settings.background.fit.repeat", { defaultValue: "平铺" }) },
    ],
    [t],
  );

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <input
          type="text"
          value={
            (config.backgroundImagePath &&
              (localStorage.getItem("backgroundImageName") ||
                config.backgroundImagePath.split(/[/\\]/).pop())) ||
            t("settings.background.default", { defaultValue: "默认背景" })
          }
          readOnly
          className="min-w-0 flex-1 h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[11px] font-mono text-ink-faint truncate"
        />
        <button
          type="button"
          onClick={() => {
            void chooseBackgroundImage().then(async (path) => {
              if (!path) return;
              const originalName = path.split(/[/\\]/).pop() ?? "";
              const saved = await invoke<string>("copy_background_image", {
                sourcePath: path,
              });
              localStorage.setItem("backgroundImageName", originalName);
              onPatch({ backgroundImagePath: saved });
            });
          }}
          className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
        >
          {t("settings.background.choose", { defaultValue: "选择" })}
        </button>
        {config.backgroundImagePath && (
          <button
            type="button"
            onClick={() => {
              localStorage.removeItem("backgroundImageName");
              onPatch({ backgroundImagePath: "" });
            }}
            className="h-8 px-3 rounded-lg border border-red-400/40 text-[11px] text-red-400 hover:bg-red-400/10 transition-colors cursor-pointer"
          >
            {t("settings.background.clear", { defaultValue: "清除" })}
          </button>
        )}
      </div>
      <SlidingButtonGroup
        options={backgroundFits}
        value={config.backgroundFit ?? "cover"}
        onChange={(value: BackgroundFit) => onPatch({ backgroundFit: value })}
      />
      <RangeRow
        label={t("settings.background.dim", { defaultValue: "遮罩" })}
        value={config.backgroundDim ?? 0.25}
        min={0}
        max={1}
        step={0.01}
        format={(value) => `${Math.round(value * 100)}%`}
        onChange={(value) => onPatch({ backgroundDim: value })}
      />
      <RangeRow
        label={t("settings.background.scale", { defaultValue: "缩放" })}
        value={config.backgroundScale ?? 1}
        min={0.5}
        max={2}
        step={0.05}
        format={(value) => `${Math.round(value * 100)}%`}
        onChange={(value) => onPatch({ backgroundScale: value })}
      />
      <RangeRow
        label={t("settings.background.positionX", { defaultValue: "横向" })}
        value={config.backgroundPositionX ?? 50}
        min={0}
        max={100}
        step={1}
        format={(value) => `${value}%`}
        onChange={(value) => onPatch({ backgroundPositionX: value })}
      />
      <RangeRow
        label={t("settings.background.positionY", { defaultValue: "纵向" })}
        value={config.backgroundPositionY ?? 50}
        min={0}
        max={100}
        step={1}
        format={(value) => `${value}%`}
        onChange={(value) => onPatch({ backgroundPositionY: value })}
      />
      <RangeRow
        label={t("settings.background.blur", { defaultValue: "模糊" })}
        value={config.backgroundBlur ?? 0}
        min={0}
        max={20}
        step={1}
        format={(value) => `${value}px`}
        onChange={(value) => onPatch({ backgroundBlur: value })}
      />
    </div>
  );
}

type HsvColor = { h: number; s: number; v: number };

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function normalizeHexColor(value: string | undefined, fallback: string): string {
  const normalized = (value ?? "").trim();
  if (/^#[0-9a-fA-F]{6}$/.test(normalized)) return normalized;
  if (/^[0-9a-fA-F]{6}$/.test(normalized)) return `#${normalized}`;
  return fallback;
}

function hexToHsv(hex: string): HsvColor {
  const normalized = normalizeHexColor(hex, DEFAULT_INTERFACE_BACKGROUND).slice(1);
  const r = parseInt(normalized.slice(0, 2), 16) / 255;
  const g = parseInt(normalized.slice(2, 4), 16) / 255;
  const b = parseInt(normalized.slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  let h = 0;
  if (delta !== 0) {
    if (max === r) h = ((g - b) / delta) % 6;
    else if (max === g) h = (b - r) / delta + 2;
    else h = (r - g) / delta + 4;
    h /= 6;
    if (h < 0) h += 1;
  }
  return { h, s: max === 0 ? 0 : delta / max, v: max };
}

function hsvToHex({ h, s, v }: HsvColor): string {
  const i = Math.floor(h * 6);
  const f = h * 6 - i;
  const p = v * (1 - s);
  const q = v * (1 - f * s);
  const t = v * (1 - (1 - f) * s);
  const [r, g, b] = [
    [v, t, p],
    [q, v, p],
    [p, v, t],
    [p, q, v],
    [t, p, v],
    [v, p, q],
  ][i % 6];
  return `#${[r, g, b]
    .map((channel) =>
      Math.round(channel * 255)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
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
          className={`absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,0.15)] transition-transform duration-250 ease-[cubic-bezier(0.22,1,0.36,1)] ${
            checked ? "translate-x-[14px]" : "translate-x-0"
          }`}
        />
      </div>
    </label>
  );
}

interface RangeRowProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onChange: (value: number) => void;
}

function RangeRow({ label, value, min, max, step, format, onChange }: RangeRowProps) {
  return (
    <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
      <span className="w-9 text-[11px] text-ink-faint">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
      />
      <span className="w-10 text-right text-[11px] font-mono text-ink-soft tabular-nums">
        {format(value)}
      </span>
    </div>
  );
}

interface ShortcutRecorderProps {
  value: string;
  onChange: (value: string) => void;
}

type ShortcutMsg = { key: string; params?: Record<string, string> } | { raw: string };

function ShortcutRecorder({ value, onChange }: ShortcutRecorderProps) {
  const { t } = useTranslation();
  const [checkState, setCheckState] = useState<"idle" | "checking" | "ok" | "warning" | "error">(
    "idle",
  );
  const [checkMsg, setCheckMsg] = useState<ShortcutMsg>({
    key: "settings.shortcut.forQuickNote",
  });
  const shortcutCheckRequestId = useRef(0);
  const isMounted = useRef(true);
  const platform = shortcutPlatform();

  const resolveMsg = (msg: ShortcutMsg): string =>
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    "raw" in msg ? msg.raw : (t as any)(msg.key, msg.params);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
      shortcutCheckRequestId.current += 1;
    };
  }, []);

  const isCurrentShortcutCheck = (requestId: number) =>
    isMounted.current && requestId === shortcutCheckRequestId.current;

  const invalidateShortcutChecks = () => {
    shortcutCheckRequestId.current += 1;
  };

  const markShortcutCleared = () => {
    invalidateShortcutChecks();
    setCheckState("idle");
    setCheckMsg({ key: "settings.shortcut.cleared" });
  };

  const runShortcutCheck = async (shortcut: string, saveWhenAvailable: boolean) => {
    // 未设置是合法状态，不需要调用后端做冲突检测。
    if (!shortcut) {
      markShortcutCleared();
      return;
    }

    const requestId = shortcutCheckRequestId.current + 1;
    shortcutCheckRequestId.current = requestId;
    setCheckState("checking");
    setCheckMsg({ key: "settings.shortcut.checking" });
    try {
      const result = await checkGlobalShortcut(shortcut);
      if (!isCurrentShortcutCheck(requestId)) return;
      const conflictMsg: ShortcutMsg = {
        key: `settings.shortcut.conflict.${result.conflictType}`,
        params: { shortcut },
      };
      if (result.available) {
        setCheckState("ok");
        setCheckMsg(conflictMsg);
        if (saveWhenAvailable) {
          onChange(shortcut);
        }
      } else {
        setCheckState("warning");
        setCheckMsg(conflictMsg);
      }
    } catch (error) {
      if (!isCurrentShortcutCheck(requestId)) return;
      setCheckState("error");
      setCheckMsg(
        error instanceof Error ? { raw: error.message } : { key: "settings.shortcut.checkFailed" },
      );
    }
  };

  const recorder = useShortcutRecorder({
    onRecord: (shortcut) => {
      if (shortcut === "") {
        onChange("");
        markShortcutCleared();
      } else if (isValidGlobalShortcut(shortcut)) {
        const configString = hotkeyToConfigString(shortcut, platform);
        void runShortcutCheck(configString, true);
      } else {
        invalidateShortcutChecks();
        setCheckState("warning");
        setCheckMsg({ key: "settings.shortcut.needsModifier" });
      }
    },
  });
  const containerRef = useRef<HTMLDivElement>(null);

  const clearShortcut = () => {
    // 显式清除会保存为空值，后端据此注销旧的全局快捷键绑定。
    recorder.cancelRecording();
    onChange("");
    markShortcutCleared();
  };

  useEffect(() => {
    if (!recorder.isRecording) return;
    const handleClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        recorder.cancelRecording();
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [recorder.isRecording, recorder.cancelRecording]);

  const liveDisplay =
    recorder.isRecording && recorder.heldKeys.length > 0
      ? formatHeldKeys(recorder.heldKeys, platform)
      : null;
  const statusClass =
    checkState === "ok"
      ? "text-bamboo"
      : checkState === "warning" || checkState === "error"
        ? "text-red-400"
        : "text-ink-ghost";
  const isChecking = checkState === "checking";

  return (
    <div ref={containerRef} className="relative space-y-1.5">
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => recorder.startRecording()}
          className={`min-w-0 flex-1 h-8 px-2.5 rounded-lg border text-[12px] flex items-center gap-2 cursor-pointer transition-colors ${
            recorder.isRecording
              ? "bg-bamboo-mist/40 border-bamboo"
              : "bg-paper-warm/70 border-paper-deep/40 hover:border-paper-deep/60"
          }`}
        >
          {recorder.isRecording ? (
            <>
              <span className="flex-1 min-w-0 text-left text-bamboo truncate">
                {liveDisplay ||
                  t("settings.shortcut.pressHint", {
                    defaultValue: "按下快捷键；按 Delete 清空。",
                  })}
              </span>
              <span className="text-[10px] text-ink-faint shrink-0">
                {t("settings.shortcut.cancelHint", { defaultValue: "Esc 取消" })}
              </span>
            </>
          ) : (
            <>
              <span
                className={`flex-1 min-w-0 text-left truncate ${
                  value ? "text-ink-soft" : "text-ink-ghost"
                }`}
              >
                {value || t("settings.shortcut.notSet", { defaultValue: "未设置" })}
              </span>
              <span className="text-[10px] text-ink-ghost shrink-0">
                {t("settings.shortcut.clickToRecord", { defaultValue: "点击录制" })}
              </span>
            </>
          )}
        </button>
        <button
          type="button"
          disabled={!value || recorder.isRecording}
          onClick={clearShortcut}
          aria-label={t("settings.shortcut.clear", { defaultValue: "清除" })}
          title={t("settings.shortcut.clear", { defaultValue: "清除" })}
          className="w-8 h-8 rounded-lg border border-paper-deep/45 text-[15px] leading-none text-ink-faint hover:text-red-400 hover:bg-paper-warm/70 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
        >
          ×
        </button>
        <button
          type="button"
          disabled={!value || isChecking || recorder.isRecording}
          onClick={() => void runShortcutCheck(value, false)}
          className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
        >
          {isChecking
            ? t("settings.shortcut.checkingShort", { defaultValue: "检测中" })
            : t("settings.shortcut.check", { defaultValue: "检测" })}
        </button>
      </div>
      <p className={`min-h-4 text-[11px] ${statusClass}`}>{resolveMsg(checkMsg)}</p>
    </div>
  );
}
