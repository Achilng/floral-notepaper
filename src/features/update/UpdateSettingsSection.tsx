import { useEffect, useMemo, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { message } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SlidingButtonGroup } from "../../components/SlidingButtonGroup";
import {
  cancelUpdate,
  checkForUpdates,
  downloadUpdate,
  getMirrorChyanCdk,
  getUpdateSettings,
  getUpdateStatus,
  installUpdate,
  saveUpdateSettings,
  setMirrorChyanCdk,
} from "./api";
import {
  getInitialUpdateStatusNotice,
  getUpdateCheckCompletionNotice,
  type UpdateInlineNotice,
} from "./presentation";
import { getUpdateErrorCode, getUpdateErrorMessage } from "./updateErrors";
import type {
  DownloadSourcePreference,
  DownloadSourceUsed,
  UpdateChannel,
  UpdateDownloadProgress,
  UpdateErrorPayload,
  UpdateInstallResult,
  UpdateSettings,
  UpdateState,
} from "./types";

type BusyAction = "settings" | "checking" | "cdk" | "download" | "cancel" | "install" | null;

interface UpdateSettingsSectionProps {
  initialSettings?: UpdateSettings;
  initialStatus?: UpdateState;
  mode?: "full" | "checkOnly" | "settingsOnly";
}

type IntervalOption = string;

export function UpdateSettingsSection({
  initialSettings,
  initialStatus,
  mode = "full",
}: UpdateSettingsSectionProps) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<UpdateSettings | null>(initialSettings ?? null);
  const [status, setStatus] = useState<UpdateState | null>(initialStatus ?? null);
  const [downloadProgress, setDownloadProgress] = useState<UpdateDownloadProgress | null>(null);
  const [cdkInput, setCdkInput] = useState("");
  const [showCdk, setShowCdk] = useState(false);
  const [busyAction, setBusyAction] = useState<BusyAction>(null);
  const [notice, setNotice] = useState<UpdateInlineNotice | null>(() =>
    getInitialUpdateStatusNotice(initialStatus, t),
  );
  const latestChannelRef = useRef<UpdateChannel>(
    initialSettings?.channel ?? initialStatus?.channel ?? "stable",
  );
  const translateRef = useRef(t);

  useEffect(() => {
    translateRef.current = t;
  }, [t]);

  useEffect(() => {
    latestChannelRef.current = settings?.channel ?? status?.channel ?? latestChannelRef.current;
  }, [settings?.channel, status?.channel]);

  const sourceOptions = useMemo<Array<{ value: DownloadSourcePreference; label: string }>>(
    () => [
      {
        value: "githubFirst",
        label: t("settings.update.source.github", { defaultValue: "GitHub" }),
      },
      {
        value: "mirrorChyanFirst",
        label: t("settings.update.source.mirrorChyan", { defaultValue: "Mirror酱" }),
      },
    ],
    [t],
  );

  const intervalOptions = useMemo<Array<{ value: IntervalOption; label: string }>>(() => {
    const options = [
      {
        value: "24",
        label: t("settings.update.interval.daily", { defaultValue: "每天" }),
      },
      {
        value: "168",
        label: t("settings.update.interval.weekly", { defaultValue: "每周" }),
      },
    ];
    const current = settings?.checkIntervalHours;
    if (current && current !== 24 && current !== 168) {
      return [
        {
          value: String(current),
          label: t("settings.update.interval.custom", {
            hours: current,
            defaultValue: "{{hours}} 小时",
          }),
        },
        ...options,
      ];
    }
    return options;
  }, [settings?.checkIntervalHours, t]);

  useEffect(() => {
    if (initialSettings && initialStatus) return;
    let alive = true;

    Promise.all([getUpdateSettings(), getUpdateStatus()])
      .then(([loadedSettings, loadedStatus]) => {
        if (!alive) return;
        latestChannelRef.current = loadedSettings.channel ?? loadedStatus.channel;
        setSettings(loadedSettings);
        setStatus(loadedStatus);
        setNotice(getInitialUpdateStatusNotice(loadedStatus, t));
      })
      .catch((error) => {
        if (!alive) return;
        setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) });
      });

    return () => {
      alive = false;
    };
  }, [initialSettings, initialStatus, t]);

  useEffect(() => {
    let active = true;

    const bindEvents = async () => {
      const unlistenFns: UnlistenFn[] = [];
      const disposeAll = () => {
        for (const unlisten of unlistenFns.splice(0)) {
          unlisten();
        }
      };

      try {
        unlistenFns.push(
          await listen<UpdateState>("update://checking", (event) => {
            if (!active) return;
            setStatus(event.payload);
          }),
        );

        unlistenFns.push(
          await listen<UpdateState>("update://checked", (event) => {
            if (!active) return;
            latestChannelRef.current = event.payload.channel;
            setStatus(event.payload);
            const nextNotice = getUpdateCheckCompletionNotice(event.payload, translateRef.current);
            if (nextNotice) {
              setNotice(nextNotice);
            }
          }),
        );

        unlistenFns.push(
          await listen<UpdateDownloadProgress>("update://download-progress", (event) => {
            if (!active) return;
            setDownloadProgress(event.payload);
            setStatus((current) =>
              deriveDownloadProgressState(current, event.payload, latestChannelRef.current),
            );
          }),
        );

        unlistenFns.push(
          await listen<UpdateState>("update://download-finished", (event) => {
            if (!active) return;
            latestChannelRef.current = event.payload.channel;
            setDownloadProgress(null);
            setStatus(event.payload);
          }),
        );

        unlistenFns.push(
          await listen<UpdateState>("update://install-finished", (event) => {
            if (!active) return;
            latestChannelRef.current = event.payload.channel;
            setStatus(event.payload);
          }),
        );

        unlistenFns.push(
          await listen<UpdateErrorPayload>("update://error", (event) => {
            if (!active) return;
            const t = translateRef.current;
            const errorText = getUpdateErrorMessage(event.payload, t);
            setNotice({ tone: "error", text: errorText });
            if (event.payload.code.startsWith("updateInstall")) {
              void message(errorText, {
                title: t("settings.update.installFailedTitle", {
                  defaultValue: "安装更新失败",
                }),
                kind: "error",
              });
            }
          }),
        );

        unlistenFns.push(
          await listen<UpdateErrorPayload>("update://auto-check-error", (event) => {
            if (!active) return;
            setNotice({
              tone: "error",
              text: getUpdateErrorMessage(event.payload, translateRef.current),
            });
          }),
        );

        return disposeAll;
      } catch (error) {
        disposeAll();
        console.error("failed to bind update settings event listeners", error);
        return () => undefined;
      }
    };

    const promise = bindEvents();

    return () => {
      active = false;
      void promise
        .then((dispose) => dispose())
        .catch((error) => {
          console.error("failed to dispose update settings event listeners", error);
        });
    };
  }, []);

  const currentVersion = status?.currentVersion ?? "--";
  const showCheckControls = mode !== "settingsOnly";
  const showSettingsControls = mode !== "checkOnly";
  const intervalValue: IntervalOption = String(settings?.checkIntervalHours ?? 24);
  const isDownloading = status?.status === "downloading";
  const isInstalling = status?.status === "installing";
  const controlsDisabled = busyAction !== null || isDownloading;
  const canCancel = isDownloading && busyAction !== "cancel";
  const currentSource = downloadProgress?.source ?? status?.source ?? null;
  const totalBytes = downloadProgress?.totalBytes ?? status?.assetSize ?? null;
  const downloadedBytes =
    downloadProgress?.downloadedBytes ??
    (status?.status === "downloaded" ? (status.assetSize ?? null) : null);
  const percent =
    downloadProgress?.percent ??
    (status?.status === "downloaded" ? 100 : status?.status === "downloading" ? 0 : null);
  const canDownload =
    status?.latestVersion != null &&
    status?.assetName != null &&
    status?.assetSize != null &&
    status?.status !== "downloaded" &&
    status?.status !== "installScheduled" &&
    !isDownloading;
  const canInstall =
    status?.latestVersion != null &&
    status?.assetPath != null &&
    status?.assetSize != null &&
    (status?.status === "downloaded" ||
      status?.status === "installScheduled" ||
      status?.status === "failed") &&
    status?.lastError?.action !== "retryDownload" &&
    !isInstalling;

  const persistSettings = async (nextSettings: UpdateSettings) => {
    latestChannelRef.current = nextSettings.channel;
    setSettings(nextSettings);
    setBusyAction("settings");
    setNotice(null);
    try {
      const saved = await saveUpdateSettings(nextSettings);
      setSettings(saved);
      setNotice({
        tone: "success",
        text: t("settings.update.saved", { defaultValue: "更新设置已保存" }),
      });
    } catch (error) {
      setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) });
    } finally {
      setBusyAction(null);
    }
  };

  const updateSettings = <Key extends keyof UpdateSettings>(
    key: Key,
    value: UpdateSettings[Key],
  ) => {
    if (!settings) return;
    void persistSettings({ ...settings, [key]: value });
  };

  const handleIntervalChange = (value: IntervalOption) => {
    if (!settings) return;
    void persistSettings({
      ...settings,
      autoCheck: true,
      checkIntervalHours: Number(value),
    });
  };

  // Auto-save CDK with debounce when the user types a non-empty value.
  // Flushes on unmount so the CDK is never lost when the user closes
  // the settings panel before the debounce fires.
  const cdkSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const cdkSavedValue = useRef("");
  const pendingCdkValue = useRef("");

  useEffect(() => {
    pendingCdkValue.current = cdkInput.trim();
  }, [cdkInput]);

  useEffect(() => {
    const value = cdkInput.trim();
    if (!value || value === cdkSavedValue.current) return;
    if (cdkSaveTimer.current) clearTimeout(cdkSaveTimer.current);
    cdkSaveTimer.current = setTimeout(() => {
      cdkSavedValue.current = value;
      setBusyAction("cdk");
      setMirrorChyanCdk(value)
        .then(() => getUpdateSettings())
        .then((saved) => setSettings(saved))
        .catch((error) => setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) }))
        .finally(() => setBusyAction(null));
    }, 600);
    return () => {
      if (cdkSaveTimer.current) clearTimeout(cdkSaveTimer.current);
    };
  }, [cdkInput, t]);

  // Flush pending CDK value on unmount so no typed input is lost.
  useEffect(() => {
    return () => {
      const pending = pendingCdkValue.current;
      if (!pending || pending === cdkSavedValue.current) return;
      cdkSavedValue.current = pending;
      setMirrorChyanCdk(pending).catch(() => {});
    };
  }, []);

  // When the settings panel opens and a CDK is already saved, fetch the
  // plaintext from the keyring so the input is pre-filled (masked by the
  // password field type) and fully editable.
  useEffect(() => {
    if (!settings?.hasMirrorChyanCdk) return;
    let cancelled = false;
    getMirrorChyanCdk().then((cdk) => {
      if (cancelled || !cdk) return;
      setCdkInput(cdk);
      cdkSavedValue.current = cdk;
    });
    return () => {
      cancelled = true;
    };
  }, [settings?.hasMirrorChyanCdk]);

  const handleCheck = async () => {
    setBusyAction("checking");
    setNotice(null);
    try {
      const result = await checkForUpdates(true);
      setNotice({
        tone: "success",
        text:
          result.status === "available"
            ? t("settings.update.available", {
                version: result.latestVersion,
                defaultValue: "发现新版本 {{version}}",
              })
            : t("settings.update.notAvailable", {
                defaultValue: "当前已是最新版本",
              }),
      });
    } catch (error) {
      setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) });
    } finally {
      try {
        setStatus(await getUpdateStatus());
      } catch (refreshError) {
        console.warn("Failed to refresh update status after check", refreshError);
      }
      setBusyAction(null);
    }
  };

  const handleDownload = async () => {
    setBusyAction("download");
    setNotice(null);
    try {
      const result = await downloadUpdate(status?.source ?? undefined);
      setNotice({
        tone: "success",
        text: t("settings.update.downloaded", {
          version: result.version ?? status?.latestVersion ?? "--",
          defaultValue: "版本 {{version}} 已下载完成",
        }),
      });
    } catch (error) {
      const message = getUpdateErrorMessage(error, t);
      const cancelled = getUpdateErrorCode(error) === "updateDownloadCancelled";
      setNotice({
        tone: cancelled ? "idle" : "error",
        text: cancelled ? t("settings.update.cancelled", { defaultValue: "下载已取消" }) : message,
      });
    } finally {
      setDownloadProgress(null);
      try {
        setStatus(await getUpdateStatus());
      } catch (refreshError) {
        console.warn("Failed to refresh update status after download", refreshError);
      }
      setBusyAction(null);
    }
  };

  const handleCancel = async () => {
    setBusyAction("cancel");
    setNotice(null);
    try {
      await cancelUpdate();
      setNotice({
        tone: "idle",
        text: t("settings.update.cancelRequested", {
          defaultValue: "已请求取消下载",
        }),
      });
    } catch (error) {
      setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) });
    } finally {
      setBusyAction(null);
    }
  };

  const handleInstall = async () => {
    const confirmed = window.confirm(
      t("settings.update.confirmInstall", {
        defaultValue:
          "应用会先保存所有未保存内容，然后关闭、安装更新，并在完成后重新打开。是否继续？",
      }),
    );
    if (!confirmed) return;

    setBusyAction("install");
    setNotice(null);
    try {
      const result = await installUpdate();
      setNotice({
        tone: "success",
        text: getInstallSuccessMessage(result, t),
      });
    } catch (error) {
      setNotice({ tone: "error", text: getUpdateErrorMessage(error, t) });
    } finally {
      try {
        setStatus(await getUpdateStatus());
      } catch (refreshError) {
        console.warn("Failed to refresh update status after install", refreshError);
      }
      setBusyAction(null);
    }
  };

  return (
    <section className="space-y-3 pt-2 border-t border-paper-deep/25">
      {showCheckControls ? (
        <>
          <div className="flex items-center justify-between gap-2">
            <div>
              <h3 className="text-[11px] font-body text-ink-faint">
                {t("settings.update.title", { defaultValue: "更新" })}
              </h3>
              <p className="mt-1 text-[10px] font-mono text-ink-ghost">
                {busyAction === "checking" || status?.status === "checking"
                  ? t("settings.update.checking", {
                      defaultValue: "正在检查...",
                    })
                  : notice
                    ? notice.text
                    : t("settings.update.currentVersion", {
                        version: currentVersion,
                        defaultValue: "当前版本：{{version}}",
                      })}
              </p>
            </div>
            <button
              type="button"
              disabled={controlsDisabled}
              onClick={() => void handleCheck()}
              className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
            >
              {busyAction === "checking"
                ? t("settings.update.busy", { defaultValue: "处理中" })
                : t("settings.update.check", { defaultValue: "检查更新" })}
            </button>
          </div>

          {renderDownloadCard({
            t,
            status,
            source: currentSource,
            totalBytes,
            downloadedBytes,
            percent,
            bytesPerSecond: downloadProgress?.bytesPerSecond ?? 0,
            canDownload,
            canCancel,
            canInstall,
            installBusy: busyAction === "install",
            isInstalling,
            onDownload: () => void handleDownload(),
            onCancel: () => void handleCancel(),
            onInstall: () => void handleInstall(),
          })}
        </>
      ) : null}

      {showSettingsControls ? (
        settings ? (
          <>
            {!showCheckControls ? (
              <div>
                <h3 className="text-[11px] font-body text-ink-faint">
                  {t("settings.update.settingsTitle", {
                    defaultValue: "更新设置",
                  })}
                </h3>
              </div>
            ) : null}

            <div className="space-y-2">
              <UpdateToggleRow
                label={t("settings.update.autoCheck", {
                  defaultValue: "自动检查更新",
                })}
                checked={settings.autoCheck}
                disabled={controlsDisabled}
                onChange={(checked) => updateSettings("autoCheck", checked)}
              />
              <UpdateToggleRow
                label={t("settings.update.autoDownload", {
                  defaultValue: "有新版本时自动下载",
                })}
                checked={settings.autoDownload}
                disabled={controlsDisabled}
                onChange={(checked) => updateSettings("autoDownload", checked)}
              />
            </div>

            <div className="space-y-2">
              <label className="block text-[11px] font-body text-ink-faint">
                {t("settings.update.interval.label", {
                  defaultValue: "检查频率",
                })}
              </label>
              <SlidingButtonGroup
                options={intervalOptions}
                value={intervalValue}
                onChange={handleIntervalChange}
              />
            </div>

            <div className="space-y-2">
              <label className="block text-[11px] font-body text-ink-faint">
                {t("settings.update.source.label", { defaultValue: "下载源" })}
              </label>
              <SlidingButtonGroup
                options={sourceOptions}
                value={settings.downloadSourcePreference}
                onChange={(value) => updateSettings("downloadSourcePreference", value)}
                className="grid grid-cols-2"
              />
            </div>

            <div className="space-y-2">
              <label className="block text-[11px] font-body text-ink-faint">
                {t("settings.update.mirrorChyan.title", {
                  defaultValue: "Mirror酱",
                })}
              </label>
              <div className="flex gap-2">
                <div className="flex-1 relative">
                  <input
                    type={showCdk && cdkInput ? "text" : "password"}
                    value={cdkInput}
                    onChange={(event) => setCdkInput(event.target.value)}
                    placeholder={t("settings.update.mirrorChyan.placeholder", {
                      defaultValue: "点击输入文本",
                    })}
                    className="min-w-0 w-full h-8 px-2.5 pr-8 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] font-mono text-ink-soft outline-none focus:border-bamboo/40 focus:bg-paper-warm/90 transition-colors"
                  />
                </div>
                <button
                  type="button"
                  onClick={() => setShowCdk((prev) => !prev)}
                  disabled={!cdkInput}
                  className="h-8 w-8 flex items-center justify-center rounded-lg border border-paper-deep/45 text-ink-ghost hover:text-ink-faint hover:bg-paper-warm disabled:opacity-30 disabled:cursor-not-allowed transition-colors cursor-pointer shrink-0"
                  title={
                    showCdk
                      ? t("settings.update.mirrorChyan.hideCdk", {
                          defaultValue: "隐藏 CDK",
                        })
                      : t("settings.update.mirrorChyan.showCdk", {
                          defaultValue: "显示 CDK",
                        })
                  }
                >
                  {showCdk ? (
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
                      <line x1="1" y1="1" x2="23" y2="23" />
                    </svg>
                  ) : (
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                      <circle cx="12" cy="12" r="3" />
                    </svg>
                  )}
                </button>
              </div>
              <p className="text-[10px] text-ink-ghost leading-relaxed">
                {t("settings.update.mirrorChyan.noCdkHint", {
                  defaultValue: "没有或者忘记 CDK？",
                })}{" "}
                <button
                  type="button"
                  onClick={() => void openUrl("https://mirrorchyan.com")}
                  className="inline-flex items-center gap-0.5 text-ink-faint hover:text-bamboo cursor-pointer transition-colors"
                >
                  Mirror酱
                  <svg
                    className="w-[1em] h-[1em]"
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
                {t("settings.update.mirrorChyan.noCdkHintSuffix", {
                  defaultValue: " 官网即刻获取。",
                })}
              </p>
            </div>
          </>
        ) : (
          <>
            {!showCheckControls ? (
              <div>
                <h3 className="text-[11px] font-body text-ink-faint">
                  {t("settings.update.settingsTitle", {
                    defaultValue: "更新设置",
                  })}
                </h3>
              </div>
            ) : null}
            <p className="text-[11px] text-ink-ghost">
              {t("settings.update.loading", {
                defaultValue: "正在读取更新设置...",
              })}
            </p>
          </>
        )
      ) : null}
    </section>
  );
}

interface DownloadCardProps {
  t: ReturnType<typeof useTranslation>["t"];
  status: UpdateState | null;
  source: DownloadSourceUsed | null;
  totalBytes: number | null;
  downloadedBytes: number | null;
  percent: number | null;
  bytesPerSecond: number;
  canDownload: boolean;
  canCancel: boolean;
  canInstall: boolean;
  installBusy: boolean;
  isInstalling: boolean;
  onDownload: () => void;
  onCancel: () => void;
  onInstall: () => void;
}

function renderDownloadCard({
  t,
  status,
  source,
  totalBytes,
  downloadedBytes,
  percent,
  bytesPerSecond,
  canDownload,
  canCancel,
  canInstall,
  installBusy,
  isInstalling,
  onDownload,
  onCancel,
  onInstall,
}: DownloadCardProps) {
  if (
    !status?.latestVersion &&
    status?.status !== "downloading" &&
    status?.status !== "downloaded"
  ) {
    return null;
  }

  const sourceLabel = getSourceLabel(source, t);
  const progressWidth = `${Math.max(0, Math.min(percent ?? 0, 100))}%`;

  return (
    <div className="space-y-2 rounded-lg border border-paper-deep/25 bg-paper-warm/40 px-3 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <p className="text-[11px] font-body text-ink-faint">
            {t("settings.update.latestVersion", {
              version: status?.latestVersion ?? "--",
              defaultValue: "待更新版本：{{version}}",
            })}
          </p>
          {status?.assetName ? (
            <p className="text-[10px] font-mono text-ink-ghost break-all">{status.assetName}</p>
          ) : null}
        </div>
        {sourceLabel ? (
          <span className="shrink-0 rounded-full border border-paper-deep/30 px-2 py-0.5 text-[10px] font-mono text-ink-ghost">
            {sourceLabel}
          </span>
        ) : null}
      </div>

      {status?.status === "downloading" || status?.status === "downloaded" ? (
        <div className="space-y-1.5">
          <div className="h-2 overflow-hidden rounded-full bg-paper-deep/15">
            <div
              className="h-full rounded-full bg-bamboo transition-[width] duration-200"
              style={{ width: progressWidth }}
            />
          </div>
          <div className="flex items-center justify-between gap-2 text-[10px] font-mono text-ink-ghost">
            <span>
              {formatBytes(downloadedBytes)}
              {totalBytes ? ` / ${formatBytes(totalBytes)}` : ""}
            </span>
            <span>{percent == null ? "--" : `${percent.toFixed(1)}%`}</span>
          </div>
          {status?.status === "downloading" ? (
            <p className="text-[10px] font-mono text-ink-ghost">
              {t("settings.update.speed", {
                speed: formatBytes(bytesPerSecond) + "/s",
                defaultValue: "速度：{{speed}}",
              })}
            </p>
          ) : null}
          {status?.status === "downloaded" && status.assetPath ? (
            <p className="text-[10px] font-mono text-ink-ghost break-all">{status.assetPath}</p>
          ) : null}
        </div>
      ) : null}

      {status?.status === "installing" || status?.status === "installScheduled" ? (
        <div className="space-y-1.5 rounded-xl bg-cloud/55 px-2.5 py-2">
          <p className="text-[10px] font-mono text-ink-ghost">
            {status.status === "installing"
              ? t("settings.update.installPreparing", {
                  defaultValue: "正在准备退出应用并安装更新...",
                })
              : t("settings.update.installScheduled", {
                  defaultValue: "检测到旧版待安装状态，请重新点击“安装并重启”完成替换",
                })}
          </p>
          {status.installLogPath ? (
            <p className="text-[10px] font-mono text-ink-ghost break-all">
              {status.installLogPath}
            </p>
          ) : null}
        </div>
      ) : null}

      {status?.status === "failed" && status.installLogPath ? (
        <div className="space-y-1.5 rounded-xl bg-paper-warm/40 px-2.5 py-2">
          <p className="text-[10px] font-mono text-ink-ghost">
            {t("settings.update.installFailed", {
              defaultValue: "最近一次安装失败，可查看日志后重试或重新下载",
            })}
          </p>
          <p className="text-[10px] font-mono text-ink-ghost break-all">{status.installLogPath}</p>
        </div>
      ) : null}

      <div className="flex gap-2">
        {canDownload ? (
          <button
            type="button"
            onClick={onDownload}
            className="h-8 px-3 rounded-lg bg-bamboo text-[11px] text-paper hover:bg-bamboo-light transition-colors cursor-pointer"
          >
            {t("settings.update.download", { defaultValue: "下载更新" })}
          </button>
        ) : null}
        {canCancel ? (
          <button
            type="button"
            onClick={onCancel}
            className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-red-400 hover:bg-danger-bg transition-colors cursor-pointer"
          >
            {t("settings.update.cancel", { defaultValue: "取消下载" })}
          </button>
        ) : null}
        {canInstall ? (
          <button
            type="button"
            disabled={installBusy || isInstalling}
            onClick={onInstall}
            className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            {status?.status === "failed"
              ? t("settings.update.installRetry", {
                  defaultValue: "重新尝试安装",
                })
              : t("settings.update.install", { defaultValue: "安装并重启" })}
          </button>
        ) : null}
      </div>
    </div>
  );
}

interface UpdateToggleRowProps {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

function UpdateToggleRow({ label, checked, disabled = false, onChange }: UpdateToggleRowProps) {
  return (
    <label
      className={`flex items-center justify-between h-9 rounded-lg px-2.5 bg-paper-warm/45 border border-paper-deep/25 ${
        disabled ? "opacity-60 cursor-not-allowed" : "cursor-pointer"
      }`}
    >
      <span className="text-[12px] text-ink-soft">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
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

function getSourceLabel(
  source: DownloadSourceUsed | null | undefined,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (source === "mirrorChyan") {
    return t("settings.update.source.mirrorChyan", { defaultValue: "MirrorChyan" });
  }
  if (source === "github") {
    return t("settings.update.source.github", { defaultValue: "GitHub" });
  }
  return null;
}

export function deriveDownloadProgressState(
  current: UpdateState | null,
  progress: UpdateDownloadProgress,
  fallbackChannel: UpdateChannel,
): UpdateState {
  if (current) {
    return {
      ...current,
      status: "downloading",
      latestVersion: progress.version,
      assetName: progress.assetName,
      assetSize: progress.totalBytes ?? current.assetSize ?? null,
      source: progress.source,
      assetPath: null,
      downloadedAt: null,
      lastError: null,
    };
  }

  return {
    status: "downloading",
    currentVersion: "--",
    latestVersion: progress.version,
    channel: fallbackChannel,
    assetName: progress.assetName,
    assetPath: null,
    assetSha256: null,
    assetSize: progress.totalBytes ?? null,
    source: progress.source,
    checkedAt: null,
    downloadedAt: null,
    lastError: null,
  };
}

function formatBytes(value: number | null | undefined) {
  if (value == null) return "--";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function getInstallSuccessMessage(
  result: UpdateInstallResult,
  t: ReturnType<typeof useTranslation>["t"],
): string {
  const tr = t as (key: string, options?: Record<string, unknown>) => string;
  if (result.mode === "test") {
    return tr("settings.update.installValidatedTest", {
      defaultValue: "安装 helper 已完成 test 模式校验",
    });
  }

  return tr("settings.update.installValidated", {
    defaultValue: "即将退出应用并安装更新，完成后会自动重新打开",
  });
}
