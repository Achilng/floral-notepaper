import { t, type TFunction } from "i18next";

interface SerializedUpdateError {
  code?: unknown;
  message?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function getUpdateErrorMessage(error: unknown, translate: TFunction = t): string {
  const code = isRecord(error) && typeof error.code === "string" ? error.code : undefined;
  const message = isRecord(error) && typeof error.message === "string" ? error.message : undefined;

  switch (code) {
    case "mirrorCdkEmpty":
      return translate("settings.update.error.cdkEmpty", {
        defaultValue: "Mirror 酱 CDK 不能为空",
      });
    case "updateSecureStoreUnavailable":
      return translate("settings.update.error.secureStoreUnavailable", {
        defaultValue: "系统安全存储暂不可用，请稍后重试",
      });
    case "updateAlreadyRunning":
      return translate("settings.update.error.alreadyRunning", {
        defaultValue: "已有更新任务正在运行",
      });
    case "updateSourceNotConfigured":
      return translate("settings.update.error.sourceNotConfigured", {
        defaultValue: "更新源尚未配置，当前阶段仅支持本地测试清单注入",
      });
    case "updateProviderFixtureUnreadable":
      return translate("settings.update.error.providerFixtureUnreadable", {
        defaultValue: "无法读取本地更新测试清单",
      });
    case "updatePlatformUnsupported":
      return translate("settings.update.error.platformUnsupported", {
        defaultValue: "当前平台或安装形态暂不支持应用内更新",
      });
    case "updateDownloadUnavailable":
      return translate("settings.update.error.downloadUnavailable", {
        defaultValue: "下载基础设施尚未启用",
      });
    case "updateDownloadNotReady":
      return translate("settings.update.error.downloadNotReady", {
        defaultValue: "当前没有可下载的更新包",
      });
    case "updateDownloadManifestUnavailable":
      return translate("settings.update.error.downloadManifestUnavailable", {
        defaultValue: "当前阶段未配置 GitHub 更新清单，无法下载更新包",
      });
    case "updateDownloadManifestUnreadable":
      return translate("settings.update.error.downloadManifestUnreadable", {
        defaultValue: "无法读取 GitHub 更新清单",
      });
    case "updateDownloadUrlInvalid":
      return translate("settings.update.error.downloadUrlInvalid", {
        defaultValue: "下载地址无效或未使用 HTTPS",
      });
    case "updateDownloadUrlNotAllowed":
      return translate("settings.update.error.downloadUrlNotAllowed", {
        defaultValue: "下载地址不在允许列表中",
      });
    case "updateDownloadSizeMismatch":
      return translate("settings.update.error.downloadSizeMismatch", {
        defaultValue: "下载文件大小校验失败",
      });
    case "updateDownloadHashMismatch":
      return translate("settings.update.error.downloadHashMismatch", {
        defaultValue: "下载文件哈希校验失败",
      });
    case "updateDownloadHttpStatus":
      return translate("settings.update.error.downloadHttpStatus", {
        defaultValue: "下载请求失败，请稍后重试",
      });
    case "updateMirrorDownloadUnavailable":
      return translate("settings.update.error.mirrorDownloadUnavailable", {
        defaultValue: "Mirror 下载源尚未配置，当前阶段请改用 GitHub 下载",
      });
    case "updateDownloadCancelled":
      return translate("settings.update.cancelled", {
        defaultValue: "下载已取消",
      });
    case "updateDownloadSourceInvalid":
      return translate("settings.update.error.downloadSourceInvalid", {
        defaultValue: "无效的下载源参数",
      });
    case "updateDownloadTaskJoinFailed":
      return translate("settings.update.error.downloadTaskJoinFailed", {
        defaultValue: "下载任务执行失败",
      });
    case "updateInstallUnavailable":
      return translate("settings.update.error.installUnavailable", {
        defaultValue: "安装调度尚未启用",
      });
    case "updateInstallNotReady":
      return translate("settings.update.error.installNotReady", {
        defaultValue: "当前没有可安装的更新包",
      });
    case "updateHelperNotFound":
      return translate("settings.update.error.helperNotFound", {
        defaultValue: "找不到更新安装助手可执行文件",
      });
    case "updateInstallSpawnFailed":
      return translate("settings.update.error.installSpawnFailed", {
        defaultValue: "启动更新安装助手失败",
      });
    case "updateInstallTaskJoinFailed":
      return translate("settings.update.error.installTaskJoinFailed", {
        defaultValue: "安装任务执行失败",
      });
    case "updateInstallHelperInvalidArguments":
      return translate("settings.update.error.installHelperInvalidArguments", {
        defaultValue: "更新安装助手参数无效",
      });
    case "updateInstallAssetMissing":
      return translate("settings.update.error.installAssetMissing", {
        defaultValue: "更新包文件不存在或无法读取",
      });
    case "updateInstallAssetSizeMismatch":
      return translate("settings.update.error.installAssetSizeMismatch", {
        defaultValue: "更新包大小校验失败",
      });
    case "updateInstallAssetHashMismatch":
      return translate("settings.update.error.installAssetHashMismatch", {
        defaultValue: "更新包哈希校验失败",
      });
    case "updateInstallTargetMissing":
      return translate("settings.update.error.installTargetMissing", {
        defaultValue: "当前安装目标不存在，无法继续",
      });
    case "updateInstallLogWriteFailed":
      return translate("settings.update.error.installLogWriteFailed", {
        defaultValue: "无法写入安装日志",
      });
    case "updateInstallHelperFailed":
      return translate("settings.update.error.installHelperFailed", {
        defaultValue: "更新安装助手执行失败",
      });
    case "updateCancelUnavailable":
      return translate("settings.update.error.cancelUnavailable", {
        defaultValue: "当前没有可取消的更新任务",
      });
    default:
      if (message) return message;
      if (error && typeof error === "object" && "message" in error) {
        return String((error as SerializedUpdateError).message);
      }
      return translate("common.operationFailed", { defaultValue: "操作失败" });
  }
}
