import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { checkGlobalShortcut, chooseBackgroundImage } from "../features/settings/api";
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
import { SUPPORTED_LOCALES } from "../locales/locale-whitelist";
import { SlidingButtonGroup } from "./SlidingButtonGroup";

const HARMONY_FONT_LICENSE_URL = new URL("../assets/fonts/LICENSE_Fonts", import.meta.url).href;

interface SettingsPanelProps {
  config: AppConfig;
  onChange: (config: AppConfig) => void;
  onChooseNotesDir: () => void;
  onClose: () => void;
}

export function SettingsPanel({ config, onChange, onChooseNotesDir, onClose }: SettingsPanelProps) {
  const { t } = useTranslation();
  const setConfigValue = <Key extends keyof AppConfig>(key: Key, value: AppConfig[Key]) => {
    onChange({ ...config, [key]: value });
  };
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
  const backgroundFits = useMemo<Array<{ value: BackgroundFit; label: string }>>(
    () => [
      { value: "cover", label: t("settings.background.fit.cover", { defaultValue: "填充" }) },
      { value: "contain", label: t("settings.background.fit.contain", { defaultValue: "完整" }) },
      { value: "repeat", label: t("settings.background.fit.repeat", { defaultValue: "平铺" }) },
    ],
    [t],
  );
  const localeOptions = useMemo(
    () =>
      SUPPORTED_LOCALES.map((locale) => ({
        value: locale,
        label:
          locale === "zh-CN"
            ? t("settings.locale.zhCN", { defaultValue: "简体中文" })
            : locale === "en-US"
              ? t("settings.locale.enUS", { defaultValue: "English" })
              : t("settings.locale.zhHK", { defaultValue: "繁體中文" }),
      })),
    [t],
  );

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
            {t("settings.notesDir", { defaultValue: "笔记目录" })}
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
            label={t("settings.tileRenderMarkdown", { defaultValue: "磁贴渲染 Markdown" })}
            checked={config.tileRenderMarkdown}
            onChange={(checked) => setConfigValue("tileRenderMarkdown", checked)}
          />
          <ToggleRow
            label={t("settings.renderHtmlMarkdown", { defaultValue: "允许 HTML 标签渲染" })}
            checked={config.renderHtmlMarkdown}
            onChange={(checked) => setConfigValue("renderHtmlMarkdown", checked)}
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
            {t("settings.fontSize.editor", { defaultValue: "编辑器字号" })}
          </label>
          <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
            <input
              type="range"
              min={8}
              max={30}
              step={1}
              value={config.fontSize ?? 14}
              onChange={(event) => setConfigValue("fontSize", Number(event.target.value))}
              className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
            />
            <span className="text-[12px] font-mono text-ink-soft tabular-nums w-8 text-right">
              {config.fontSize ?? 14}px
            </span>
          </div>
        </section>

        <section className="space-y-2">
          <label className="block text-[11px] font-body text-ink-faint">
            {t("settings.fontSize.surface", { defaultValue: "小窗/磁贴字号" })}
          </label>
          <div className="flex items-center gap-3 h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25">
            <input
              type="range"
              min={8}
              max={30}
              step={1}
              value={config.surfaceFontSize ?? 14}
              onChange={(event) => setConfigValue("surfaceFontSize", Number(event.target.value))}
              className="flex-1 h-1 accent-bamboo cursor-pointer appearance-none bg-transparent [&::-webkit-slider-runnable-track]:h-[3px] [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-paper-deep/50 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-bamboo [&::-webkit-slider-thumb]:-mt-[4.5px] [&::-webkit-slider-thumb]:shadow-[0_1px_3px_rgba(0,0,0,0.15)]"
            />
            <span className="text-[12px] font-mono text-ink-soft tabular-nums w-8 text-right">
              {config.surfaceFontSize ?? 14}px
            </span>
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
            {t("settings.background.label", { defaultValue: "背景图片" })}
          </label>
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
                  setConfigValue("backgroundImagePath", saved);
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
                  setConfigValue("backgroundImagePath", "");
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
            onChange={(value: BackgroundFit) => setConfigValue("backgroundFit", value)}
          />
          <RangeRow
            label={t("settings.background.dim", { defaultValue: "遮罩" })}
            value={config.backgroundDim ?? 0.25}
            min={0}
            max={1}
            step={0.01}
            format={(value) => `${Math.round(value * 100)}%`}
            onChange={(value) => setConfigValue("backgroundDim", value)}
          />
          <RangeRow
            label={t("settings.background.scale", { defaultValue: "缩放" })}
            value={config.backgroundScale ?? 1}
            min={0.5}
            max={2}
            step={0.05}
            format={(value) => `${Math.round(value * 100)}%`}
            onChange={(value) => setConfigValue("backgroundScale", value)}
          />
          <RangeRow
            label={t("settings.background.positionX", { defaultValue: "横向" })}
            value={config.backgroundPositionX ?? 50}
            min={0}
            max={100}
            step={1}
            format={(value) => `${value}%`}
            onChange={(value) => setConfigValue("backgroundPositionX", value)}
          />
          <RangeRow
            label={t("settings.background.positionY", { defaultValue: "纵向" })}
            value={config.backgroundPositionY ?? 50}
            min={0}
            max={100}
            step={1}
            format={(value) => `${value}%`}
            onChange={(value) => setConfigValue("backgroundPositionY", value)}
          />
          <RangeRow
            label={t("settings.background.blur", { defaultValue: "模糊" })}
            value={config.backgroundBlur ?? 0}
            min={0}
            max={20}
            step={1}
            format={(value) => `${value}px`}
            onChange={(value) => setConfigValue("backgroundBlur", value)}
          />
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

  const runShortcutCheck = async (shortcut: string, saveWhenAvailable: boolean) => {
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
        invalidateShortcutChecks();
        onChange("");
        setCheckState("idle");
        setCheckMsg({ key: "settings.shortcut.cleared" });
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
          disabled={isChecking || recorder.isRecording}
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
