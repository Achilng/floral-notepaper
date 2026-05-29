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
    case "updateCheckTaskJoinFailed":
      return translate("settings.update.error.checkTaskJoinFailed", {
        defaultValue: "检查更新任务执行失败",
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
    case "updateInstallSaveFailed":
      return translate("settings.update.error.installSaveFailed", {
        defaultValue: "安装前自动保存失败，请先处理当前未保存内容后重试",
      });
    case "updateInstallSaveTimedOut":
      return translate("settings.update.error.installSaveTimedOut", {
        defaultValue: "等待窗口保存未保存内容超时，请稍后重试",
      });
    case "updateInstallHelperHandshakeFailed":
      return translate("settings.update.error.installHelperHandshakeFailed", {
        defaultValue: "更新安装助手未能在退出前完成就绪握手",
      });
    case "updateInstallWaitTimedOut":
      return translate("settings.update.error.installWaitTimedOut", {
        defaultValue: "等待应用退出超时，请重试安装",
      });
    case "updateInstallInsufficientSpace":
      return translate("settings.update.error.installInsufficientSpace", {
        defaultValue: "磁盘剩余空间不足，无法继续安装更新",
      });
    case "updateInstallUnsupportedKind":
      return translate("settings.update.error.installUnsupportedKind", {
        defaultValue: "当前安装形态暂不支持应用内安装",
      });
    case "updatePortableManualOnly":
      return translate("settings.update.error.portableManualOnly", {
        defaultValue: "当前便携版仅支持手动下载更新包后覆盖升级",
      });
    case "updateInstallAssetExtractFailed":
      return translate("settings.update.error.installAssetExtractFailed", {
        defaultValue: "无法解包更新资源，请重新下载后重试",
      });
    case "updateInstallReplaceFailed":
      return translate("settings.update.error.installReplaceFailed", {
        defaultValue: "替换当前安装内容失败，请稍后重试",
      });
    case "updateInstallRelaunchFailed":
      return translate("settings.update.error.installRelaunchFailed", {
        defaultValue: "更新完成后重新启动应用失败，请手动重新打开应用",
      });
    case "updateInstallStateWriteFailed":
      return translate("settings.update.error.installStateWriteFailed", {
        defaultValue: "无法写入安装状态文件",
      });
    case "updateInstallInstallerFailed":
      return translate("settings.update.error.installInstallerFailed", {
        defaultValue: "更新安装程序执行失败",
      });
    case "updateInstallInstallerTimedOut":
      return translate("settings.update.error.installInstallerTimedOut", {
        defaultValue: "更新安装程序执行超时，请稍后重试",
      });
    case "updateInstallInstallerCancelled":
      return translate("settings.update.error.installInstallerCancelled", {
        defaultValue: "更新安装已取消",
      });
    case "updateInstallInstallerBusy":
      return translate("settings.update.error.installInstallerBusy", {
        defaultValue: "另一个安装程序正在运行，请稍后重试",
      });
    case "updateInstallInstallerFatal":
      return translate("settings.update.error.installInstallerFatal", {
        defaultValue: "更新安装程序返回了致命错误",
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
    case "updateInstallVersionMismatch":
      return translate("settings.update.error.installVersionMismatch", {
        defaultValue: "安装后重新打开的仍是旧版本，请直接重试安装",
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
    case "updateGithubApi":
      return translate("settings.update.error.githubApi", {
        defaultValue: "GitHub API 请求失败，请检查网络后重试",
      });
    case "updateGithubRateLimited":
      return translate("settings.update.error.githubRateLimited", {
        defaultValue: "GitHub API 频率限制，请稍后重试",
      });
    case "updateGithubNoAssets":
      return translate("settings.update.error.githubNoAssets", {
        defaultValue: "GitHub Release 中没有找到可用资产",
      });
    default:
      if (message) return message;
      if (error && typeof error === "object" && "message" in error) {
        return String((error as SerializedUpdateError).message);
      }
      return translate("common.operationFailed", { defaultValue: "操作失败" });
  }
}
