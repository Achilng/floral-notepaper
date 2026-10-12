import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getErrorMessage } from "../notes/api";
import {
  clearFeishuSecret,
  getFeishuSettings,
  getFeishuSyncStatus,
  pollFeishuAuthorization,
  saveFeishuSettings,
  startFeishuAuthorization,
  syncAllNotesToFeishu,
  testFeishuConnection,
  type FeishuSettings,
  type FeishuDeviceAuthorization,
  type FeishuSyncOverview,
} from "./api";

type Feedback = { tone: "success" | "error" | "neutral"; message: string } | null;

const EMPTY_SETTINGS: FeishuSettings = {
  enabled: false,
  appId: "",
  folderToken: "",
  hasSecret: false,
  connected: false,
};

export function FeishuSettingsSection() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<FeishuSettings>(EMPTY_SETTINGS);
  const [secret, setSecret] = useState("");
  const [status, setStatus] = useState<FeishuSyncOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"save" | "test" | "login" | "sync" | "clear" | null>(null);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [authorization, setAuthorization] = useState<FeishuDeviceAuthorization | null>(null);

  const refreshStatus = useCallback(async () => {
    setStatus(await getFeishuSyncStatus());
  }, []);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([getFeishuSettings(), getFeishuSyncStatus()])
      .then(([nextSettings, nextStatus]) => {
        if (cancelled) return;
        setSettings(nextSettings);
        setStatus(nextStatus);
      })
      .catch((error) => {
        if (!cancelled) {
          setFeedback({ tone: "error", message: getErrorMessage(error) });
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    const unlisten = listen<FeishuSyncOverview>("feishu-sync-status", (event) => {
      if (!cancelled) setStatus(event.payload);
    });
    return () => {
      cancelled = true;
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  const persist = async () => {
    const saved = await saveFeishuSettings(settings, secret || undefined);
    setSettings(saved);
    setSecret("");
    await refreshStatus();
    return saved;
  };

  const handleSave = async () => {
    setBusy("save");
    setFeedback(null);
    try {
      await persist();
      setFeedback({
        tone: "success",
        message: t("settings.feishu.saved", { defaultValue: "飞书同步设置已保存" }),
      });
    } catch (error) {
      setFeedback({ tone: "error", message: getErrorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const handleTest = async () => {
    setBusy("test");
    setFeedback(null);
    try {
      await persist();
      await testFeishuConnection();
      setFeedback({
        tone: "success",
        message: t("settings.feishu.connectionOk", { defaultValue: "连接成功，应用凭据有效" }),
      });
    } catch (error) {
      setFeedback({ tone: "error", message: getErrorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const handleSync = async () => {
    setBusy("sync");
    setFeedback(null);
    try {
      await persist();
      const summary = await syncAllNotesToFeishu();
      await refreshStatus();
      setFeedback({
        tone: summary.failed > 0 ? "error" : "success",
        message: t("settings.feishu.syncSummary", {
          synced: summary.synced,
          skipped: summary.skipped,
          failed: summary.failed,
          defaultValue: "同步完成：{{synced}} 篇更新，{{skipped}} 篇无需更新，{{failed}} 篇失败",
        }),
      });
    } catch (error) {
      setFeedback({ tone: "error", message: getErrorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const handleLogin = async () => {
    setBusy("login");
    setFeedback(null);
    setAuthorization(null);
    try {
      await persist();
      const nextAuthorization = await startFeishuAuthorization();
      setAuthorization(nextAuthorization);
      try {
        await openUrl(nextAuthorization.verificationUriComplete);
      } catch {
        // Keep the visible link and code available when the OS cannot open it.
      }
      const deadline = Date.now() + nextAuthorization.expiresIn * 1000;
      while (Date.now() < deadline) {
        await new Promise((resolve) =>
          window.setTimeout(resolve, Math.max(nextAuthorization.interval, 3) * 1000),
        );
        const result = await pollFeishuAuthorization(nextAuthorization.deviceCode);
        if (result.connected) {
          const [nextSettings] = await Promise.all([getFeishuSettings(), refreshStatus()]);
          setSettings(nextSettings);
          setAuthorization(null);
          setFeedback({
            tone: "success",
            message: t("settings.feishu.loginOk", { defaultValue: "飞书账号授权成功" }),
          });
          return;
        }
      }
      throw new Error(t("settings.feishu.loginExpired", { defaultValue: "授权已过期，请重试" }));
    } catch (error) {
      setFeedback({ tone: "error", message: getErrorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const handleClearSecret = async () => {
    setBusy("clear");
    setFeedback(null);
    try {
      const saved = await clearFeishuSecret();
      setSettings(saved);
      setSecret("");
      await refreshStatus();
      setFeedback({
        tone: "neutral",
        message: t("settings.feishu.secretCleared", { defaultValue: "App Secret 已清除" }),
      });
    } catch (error) {
      setFeedback({ tone: "error", message: getErrorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const disabled = busy !== null || loading;
  const statusText = status?.syncing
    ? t("settings.feishu.status.syncing", { defaultValue: "正在同步" })
    : status?.configured
      ? t("settings.feishu.status.counts", {
          synced: status.synced,
          total: status.total,
          failed: status.failed,
          defaultValue: "已同步 {{synced}}/{{total}}，失败 {{failed}}",
        })
      : t("settings.feishu.status.notConfigured", { defaultValue: "尚未完成配置" });

  return (
    <section className="space-y-3 pt-3 border-t border-paper-deep/25">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-[12px] font-medium text-ink-soft">
            {t("settings.feishu.title", { defaultValue: "飞书云同步" })}
          </h3>
          <p className="mt-1 text-[10px] leading-relaxed text-ink-ghost">
            {t("settings.feishu.description", {
              defaultValue: "本地优先，保存后异步单向同步到飞书文档。飞书端修改可能被覆盖。",
            })}
          </p>
        </div>
        <label className="relative inline-flex shrink-0 cursor-pointer items-center">
          <input
            type="checkbox"
            className="peer sr-only"
            checked={settings.enabled}
            disabled={loading}
            onChange={(event) =>
              setSettings((current) => ({ ...current, enabled: event.target.checked }))
            }
          />
          <span className="h-5 w-9 rounded-full bg-paper-deep/70 transition-colors peer-checked:bg-bamboo/75 peer-disabled:opacity-50 after:absolute after:left-0.5 after:top-0.5 after:h-4 after:w-4 after:rounded-full after:bg-white after:shadow-sm after:transition-transform peer-checked:after:translate-x-4" />
        </label>
      </div>

      <div className="space-y-2">
        <label className="block text-[10px] text-ink-faint">
          {t("settings.feishu.appId", { defaultValue: "App ID" })}
          <input
            type="text"
            value={settings.appId}
            spellCheck={false}
            autoComplete="off"
            placeholder="cli_xxxxxxxxxxxxx"
            onChange={(event) =>
              setSettings((current) => ({ ...current, appId: event.target.value }))
            }
            className="mt-1 h-8 w-full rounded-lg border border-paper-deep/40 bg-paper-warm/70 px-2.5 font-mono text-[11px] text-ink-soft outline-none focus:border-bamboo/50"
          />
        </label>
        <label className="block text-[10px] text-ink-faint">
          {t("settings.feishu.appSecret", { defaultValue: "App Secret" })}
          <input
            type="password"
            value={secret}
            autoComplete="new-password"
            placeholder={
              settings.hasSecret
                ? t("settings.feishu.secretStored", { defaultValue: "已安全保存；留空表示不修改" })
                : t("settings.feishu.secretPlaceholder", { defaultValue: "输入 App Secret" })
            }
            onChange={(event) => setSecret(event.target.value)}
            className="mt-1 h-8 w-full rounded-lg border border-paper-deep/40 bg-paper-warm/70 px-2.5 font-mono text-[11px] text-ink-soft outline-none focus:border-bamboo/50"
          />
        </label>
        <label className="block text-[10px] text-ink-faint">
          {t("settings.feishu.folderToken", { defaultValue: "目标文件夹 Token（可选）" })}
          <input
            type="text"
            value={settings.folderToken}
            spellCheck={false}
            autoComplete="off"
            placeholder={t("settings.feishu.folderPlaceholder", {
              defaultValue: "飞书文件夹链接中的 token",
            })}
            onChange={(event) =>
              setSettings((current) => ({ ...current, folderToken: event.target.value }))
            }
            className="mt-1 h-8 w-full rounded-lg border border-paper-deep/40 bg-paper-warm/70 px-2.5 font-mono text-[11px] text-ink-soft outline-none focus:border-bamboo/50"
          />
        </label>
      </div>

      <div className="rounded-lg border border-paper-deep/25 bg-paper-warm/35 px-2.5 py-2">
        <div className="flex items-center justify-between gap-2 text-[10px]">
          <span className="text-ink-ghost">
            {t("settings.feishu.syncStatus", { defaultValue: "同步状态" })}
          </span>
          <span className={status?.failed ? "text-red-400" : "text-ink-faint"}>{statusText}</span>
        </div>
        {status?.lastError ? (
          <p className="mt-1 break-words text-[9px] leading-relaxed text-red-400/90">
            {status.lastError}
          </p>
        ) : null}
      </div>

      <div className="rounded-lg border border-paper-deep/25 bg-paper-warm/35 px-2.5 py-2 text-[10px]">
        <span className={settings.connected ? "text-bamboo" : "text-ink-ghost"}>
          {settings.connected
            ? t("settings.feishu.loggedIn", { defaultValue: "飞书账号已授权" })
            : t("settings.feishu.notLoggedIn", { defaultValue: "飞书账号未授权" })}
        </span>
        {authorization ? (
          <p className="mt-1 break-words leading-relaxed text-ink-faint">
            {t("settings.feishu.authCode", {
              code: authorization.userCode,
              defaultValue: "请在已打开的飞书页面确认授权，验证码：{{code}}",
            })}{" "}
            <button
              type="button"
              className="text-bamboo underline"
              onClick={() => void openUrl(authorization.verificationUriComplete)}
            >
              {t("settings.feishu.openAuth", { defaultValue: "重新打开授权页" })}
            </button>
          </p>
        ) : null}
      </div>

      {feedback ? (
        <p
          role="status"
          className={`text-[10px] leading-relaxed ${
            feedback.tone === "error"
              ? "text-red-400"
              : feedback.tone === "success"
                ? "text-bamboo"
                : "text-ink-faint"
          }`}
        >
          {feedback.message}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <ActionButton disabled={disabled} onClick={() => void handleSave()}>
          {busy === "save"
            ? t("settings.feishu.saving", { defaultValue: "保存中…" })
            : t("settings.feishu.save", { defaultValue: "保存配置" })}
        </ActionButton>
        <ActionButton disabled={disabled} onClick={() => void handleTest()}>
          {busy === "test"
            ? t("settings.feishu.testing", { defaultValue: "测试中…" })
            : t("settings.feishu.test", { defaultValue: "测试连接" })}
        </ActionButton>
        <ActionButton disabled={disabled} onClick={() => void handleLogin()}>
          {busy === "login"
            ? t("settings.feishu.loggingIn", { defaultValue: "等待授权…" })
            : t("settings.feishu.login", { defaultValue: "登录飞书" })}
        </ActionButton>
        <ActionButton
          disabled={disabled || !settings.enabled || !settings.connected}
          onClick={() => void handleSync()}
        >
          {busy === "sync"
            ? t("settings.feishu.syncing", { defaultValue: "同步中…" })
            : t("settings.feishu.syncAll", { defaultValue: "立即同步全部" })}
        </ActionButton>
        {settings.hasSecret ? (
          <button
            type="button"
            disabled={disabled}
            onClick={() => void handleClearSecret()}
            className="h-7 px-2.5 rounded-lg text-[10px] text-red-400/85 hover:bg-red-400/10 disabled:opacity-50 transition-colors cursor-pointer"
          >
            {t("settings.feishu.clearSecret", { defaultValue: "清除密钥" })}
          </button>
        ) : null}
      </div>

      <p className="text-[9px] leading-relaxed text-ink-ghost/80">
        {t("settings.feishu.permissionHint", {
          defaultValue:
            "需开通“创建及编辑新版文档”；同步到指定文件夹还需开通“查看、评论、编辑和管理云空间中所有文件”，然后发布应用新版本。",
        })}
      </p>
    </section>
  );
}

interface ActionButtonProps {
  disabled: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function ActionButton({ disabled, onClick, children }: ActionButtonProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="h-7 px-2.5 rounded-lg border border-paper-deep/45 text-[10px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
    >
      {children}
    </button>
  );
}
