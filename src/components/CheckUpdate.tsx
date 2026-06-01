import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import { confirm } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppConfig, UpdateInfo } from "../features/settings/types";

interface CheckUpdateProps {
  config: AppConfig;
}

interface RepoInfo {
  name: string;
  url: string;
}

const mirrors: string[] = ["https://api.github.com/", "https://api.kkgithub.com/"];

interface MultiplePlatformInfo {
  supported: boolean;
  executable: RegExp;
}

const platformConfig: Record<string, MultiplePlatformInfo> = {
  windows: {
    supported: true,
    executable: /^floral-notepaper_(\d+\.)+exe$/,
  },
};

export function CheckUpdate({ config }: CheckUpdateProps) {
  const { t } = useTranslation();
  const [checkingUpdate, setCheckingUpdate] = useState(false);

  // 构建仓库配置
  const getRepoInfo = (): RepoInfo => {
    const owner = config.githubOwner || "Achilng";
    const repo = config.githubRepo || "floral-notepaper";
    return {
      name: `${owner}/${repo}`,
      url: `https://api.github.com/repos/${owner}/${repo}/releases/latest`,
    };
  };

  // 使用镜像源检查更新
  const checkUpdateWithMirrors = async () => {
    let lastError: Error | null = null;
    const repoInfo = getRepoInfo();
    const [owner, repo] = repoInfo.name.split("/");

    for (const mirror of mirrors) {
      try {
        const apiUrl = `${mirror}repos/${owner}/${repo}/releases/latest`;

        const result = await invoke<UpdateInfo>("check_update_with_url", {
          config,
          apiUrl,
        });

        return result;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        lastError = new Error(`镜像源 ${mirror} 失败：${errorMsg}`);
      }
    }

    throw lastError || new Error("所有镜像源均失败");
  };

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);

    try {
      const result = await checkUpdateWithMirrors();

      if (result.hasUpdate) {
        const confirmed = await confirm(
          t("settings.update.available", {
            defaultValue: `发现新版本 ${result.version}！是否前往下载？`,
            version: result.version,
          }),
          {
            title: t("settings.update.checkNow", { defaultValue: "检查更新" }),
            okLabel: "前往下载",
            cancelLabel: "取消",
          },
        );
        if (confirmed) {
          await openUrl(result.releaseUrl);
        }
      }
    } finally {
      setCheckingUpdate(false);
    }
  };

  // 多平台处理逻辑
  const platformInfo = (() => {
    const currentPlatform = platform();
    const cfg = platformConfig[currentPlatform];

    if (!cfg) {
      return {
        supported: false,
        message: t("settings.update.platformNotSupported", {
          defaultValue: "当前平台不支持自动更新检查",
        }),
      };
    }

    return {
      supported: cfg.supported,
      message: cfg.supported
        ? null
        : t("settings.update.platformNotSupported", {
            defaultValue: "当前平台不支持自动更新检查",
          }),
    };
  })();

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={handleCheckUpdate}
        disabled={checkingUpdate || !platformInfo.supported}
        className="flex-1 h-8 rounded-lg bg-surface border border-outline/20 text-[11px] text-ink hover:bg-surface/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {checkingUpdate
          ? t("settings.update.checking", { defaultValue: "检查中..." })
          : t("settings.update.checkNow", { defaultValue: "立即检查更新" })}
      </button>
    </div>
  );
}
