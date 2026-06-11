import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { UpdateSettingsSection } from "../features/update/UpdateSettingsSection";
import contributorsData from "../generated/contributors.json";
import { getTips, parseTip } from "../locales/tips";

interface Contributor {
  login: string;
  avatar_url: string;
  html_url: string;
}

const contributors = contributorsData as Contributor[];

interface AboutPanelProps {
  onClose: () => void;
}

export function AboutPanel({ onClose }: AboutPanelProps) {
  const { t, i18n } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const tipSegments = useMemo(() => {
    const tips = getTips(i18n.language);
    const raw = tips[Math.floor(Math.random() * tips.length)] ?? "";
    return parseTip(raw);
  }, [i18n.language]);

  const tipContainerRef = useRef<HTMLDivElement>(null);
  const tipInnerRef = useRef<HTMLSpanElement>(null);
  const [tipScrollable, setTipScrollable] = useState(false);
  const [tipScrollOffset, setTipScrollOffset] = useState("0px");

  const checkTipOverflow = useCallback(() => {
    const container = tipContainerRef.current;
    const inner = tipInnerRef.current;
    if (!container || !inner) return;
    const overflow = inner.scrollWidth - container.clientWidth;
    if (overflow > 0) {
      setTipScrollable(true);
      setTipScrollOffset(`-${overflow}px`);
    } else {
      setTipScrollable(false);
    }
  }, []);

  useEffect(() => {
    requestAnimationFrame(checkTipOverflow);
  }, [tipSegments, checkTipOverflow]);

  return (
    <aside className="w-[360px] h-full shrink-0 border-l border-paper-deep/30 bg-cloud/92 backdrop-blur-sm flex flex-col">
      <div className="flex items-center justify-between h-11 px-4 border-b border-paper-deep/25">
        <h2 className="text-[13px] font-display font-medium text-ink-soft">
          {t("about.title", { defaultValue: "关于" })}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
          title={t("about.closeTitle", { defaultValue: "关闭关于" })}
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
        <section className="space-y-1.5">
          <h3 className="text-[20px] font-serif font-medium text-ink-soft">
            {t("about.productName", { defaultValue: "花笺" })}
          </h3>
          <p className="text-[11px] text-ink-ghost font-body">
            {t("about.summary", { defaultValue: "轻量、优雅、现代化的本地便签工具" })}
          </p>
          <p className="text-[11px] text-ink-ghost font-body">
            Copyright © 2026 Floral Notepaper Contributors,
            <br />
            Licensed under the MIT License. <br />
          </p>
          {version && (
            <p className="text-[11px] text-ink-ghost font-mono">
              {t("about.version", { defaultValue: "版本：v{{version}}", version })}
            </p>
          )}
        </section>

        <UpdateSettingsSection mode="checkOnly" />

        <div className="space-y-2">
          <section className="space-y-1 py-2 border-y border-paper-deep/25">
            <button
              type="button"
              onClick={() => void openUrl("https://github.com/Achilng/floral-notepaper")}
              className="w-full h-8 px-1 flex items-center justify-between text-[11px] text-ink-faint hover:text-bamboo cursor-pointer transition-colors"
            >
              <span className="inline-flex items-center gap-1.5">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  className="shrink-0"
                >
                  <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z" />
                </svg>
                <span className="leading-none">
                  {t("about.github", { defaultValue: "GitHub" })}
                </span>
              </span>
              <span className="inline-flex items-center">
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M6 3h7v7M13 3L3 13" />
                </svg>
              </span>
            </button>
            <button
              type="button"
              onClick={() => void openUrl("https://github.com/Achilng/floral-notepaper/issues")}
              className="w-full h-8 px-1 flex items-center justify-between text-[11px] text-ink-faint hover:text-bamboo cursor-pointer transition-colors"
            >
              <span className="inline-flex items-center gap-1.5">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  className="shrink-0"
                >
                  <path d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM0 8a8 8 0 1116 0A8 8 0 010 8zm9 3a1 1 0 11-2 0 1 1 0 012 0zm-.25-6.25a.75.75 0 00-1.5 0v3.5a.75.75 0 001.5 0v-3.5z" />
                </svg>
                <span className="leading-none">
                  {t("about.feedback", { defaultValue: "反馈问题" })}
                </span>
              </span>
              <span className="inline-flex items-center">
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M6 3h7v7M13 3L3 13" />
                </svg>
              </span>
            </button>
            <button
              type="button"
              onClick={() =>
                void openUrl("https://github.com/Achilng/floral-notepaper/blob/main/LICENSE")
              }
              className="w-full h-8 px-1 flex items-center justify-between text-[11px] text-ink-faint hover:text-bamboo cursor-pointer transition-colors"
            >
              <span className="inline-flex items-center gap-1.5">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  className="shrink-0"
                >
                  <path d="M4 1.5c-.83 0-1.5.67-1.5 1.5v10c0 .83.67 1.5 1.5 1.5h8c.83 0 1.5-.67 1.5-1.5V5.5L9.5 1.5H4zm0 1h5v3.5h3.5V13a.5.5 0 01-.5.5H4a.5.5 0 01-.5-.5V3a.5.5 0 01.5-.5zm5.5.707L11.793 5.5H9.5V3.207zM5.5 7.5v1h5v-1h-5zm0 2v1h5v-1h-5zm0 2v1h3v-1h-3z" />
                </svg>
                <span className="leading-none">
                  {t("about.license", { defaultValue: "许可证" })}
                </span>
              </span>
              <span className="inline-flex items-center">
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M6 3h7v7M13 3L3 13" />
                </svg>
              </span>
            </button>
          </section>

          <section className="space-y-2">
            <h3 className="text-[11px] font-body text-ink-faint">
              {t("about.contributors", { defaultValue: "贡献者" })}
            </h3>
            <div className="flex flex-wrap gap-2">
              {contributors.map((c) => (
                <button
                  key={c.login}
                  type="button"
                  onClick={() => void openUrl(c.html_url)}
                  className="flex items-center gap-1.5 group cursor-pointer rounded-md px-1 py-0.5 hover:bg-bamboo-mist/40 transition-colors"
                  title={c.login}
                >
                  <img src={c.avatar_url} alt={c.login} className="w-6 h-6 rounded-full" />
                  <span className="text-[10px] text-ink-ghost group-hover:text-bamboo transition-colors">
                    {c.login}
                  </span>
                </button>
              ))}
            </div>
          </section>
        </div>
      </div>

      <div className="flex items-center px-4 h-7 border-t border-paper-deep/20 bg-paper/30 shrink-0 overflow-hidden">
        <span className="text-[10px] text-ink-ghost font-body shrink-0 mr-1">
          {t("about.tipPrefix", { defaultValue: "您知道吗：" })}
        </span>
        <div ref={tipContainerRef} className="flex-1 overflow-hidden min-w-0 flex items-center">
          <span
            ref={tipInnerRef}
            className={`text-[10px] text-ink-ghost font-body whitespace-nowrap inline-block ${tipScrollable ? "animate-tip-marquee" : ""}`}
            style={
              tipScrollable ? { ["--tip-scroll-offset" as string]: tipScrollOffset } : undefined
            }
          >
            {tipSegments.map((seg, i) =>
              seg.type === "link" ? (
                <button
                  key={i}
                  type="button"
                  onClick={() => void openUrl(seg.url!)}
                  className="inline-flex items-center gap-0.5 text-ink-faint hover:text-bamboo cursor-pointer transition-colors"
                >
                  {seg.text}
                  <svg
                    width="9"
                    height="9"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M6 3h7v7M13 3L3 13" />
                  </svg>
                </button>
              ) : (
                <span key={i}>{seg.text}</span>
              ),
            )}
          </span>
        </div>
      </div>
    </aside>
  );
}
