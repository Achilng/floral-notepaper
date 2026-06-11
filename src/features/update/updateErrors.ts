/**
 * ja locale for settings.update.error.* is not yet translated.
 * Japanese users will see the zh-CN fallback text from the defaultValue until ja translations are added.
 */
import { t, type TFunction } from "i18next";

interface SerializedUpdateError {
  code?: unknown;
  message?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function getUpdateErrorCode(error: unknown): string | undefined {
  if (!isRecord(error)) return undefined;
  if (typeof error.code === "string") return error.code;

  for (const key of ["payload", "error", "data"]) {
    const nested = error[key];
    if (isRecord(nested) && typeof nested.code === "string") {
      return nested.code;
    }
  }

  return undefined;
}

function getUpdateErrorText(error: unknown): string | undefined {
  if (!isRecord(error)) return undefined;
  if (typeof error.message === "string") return error.message;

  for (const key of ["payload", "error", "data"]) {
    const nested = error[key];
    if (isRecord(nested) && typeof nested.message === "string") {
      return nested.message;
    }
  }

  return undefined;
}

export function getUpdateErrorMessage(error: unknown, translate: TFunction = t): string {
  const code = getUpdateErrorCode(error);
  const message = getUpdateErrorText(error);

  switch (code) {
    case "mirrorChyanCdkEmpty":
      return translate("settings.update.error.cdkEmpty", {
        defaultValue: "Mirror酱 CDK 不能为空",
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
    case "updateProviderNotConfigured":
      return translate("settings.update.error.providerNotConfigured", {
        defaultValue: "更新源尚未配置测试清单",
      });
    case "updateProviderFixtureUnreadable":
      return translate("settings.update.error.providerFixtureUnreadable", {
        defaultValue: "无法读取本地更新测试清单",
      });
    case "updateVersionInvalid":
      return translate("settings.update.error.versionInvalid", {
        defaultValue: "版本号格式无效",
      });
    case "updateManifestInvalid":
      return translate("settings.update.error.manifestInvalid", {
        defaultValue: "更新清单格式无效",
      });
    case "updateManifestAssetNotFound":
      return translate("settings.update.error.manifestAssetNotFound", {
        defaultValue: "当前平台没有匹配的更新包",
      });
    case "updateManifestUnsupportedSchema":
      return translate("settings.update.error.manifestUnsupportedSchema", {
        defaultValue: "更新清单格式版本暂不受支持",
      });
    case "updateManifestAppIdMismatch":
      return translate("settings.update.error.manifestAppIdMismatch", {
        defaultValue: "更新清单不属于当前应用",
      });
    case "updateManifestMissingAssets":
      return translate("settings.update.error.manifestMissingAssets", {
        defaultValue: "更新清单未包含任何可下载资产",
      });
    case "updateStateCorrupted":
      return translate("settings.update.error.stateCorrupted", {
        defaultValue: "更新状态文件已损坏，已重置",
      });
    case "updateStateLockTimeout":
      return translate("settings.update.error.stateLockTimeout", {
        defaultValue: "等待更新状态文件锁超时，请稍后重试",
      });
    case "updateCheckInterrupted":
      return translate("settings.update.error.checkInterrupted", {
        defaultValue: "上次检查更新被中断，已恢复为空闲状态",
      });
    case "updatePlatformUnsupported":
      return translate("settings.update.error.platformUnsupported", {
        defaultValue: "当前平台或安装形态暂不支持应用内更新",
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
    case "updateDownloadInterrupted":
      return translate("settings.update.error.downloadInterrupted", {
        defaultValue: "上次下载被中断，请重新下载更新包",
      });
    case "updateDownloadVersionMismatch":
      return translate("settings.update.error.downloadVersionMismatch", {
        defaultValue: "下载版本与已检查到的版本不一致，请重新检查更新",
      });
    case "updateDownloadAssetMismatch":
      return translate("settings.update.error.downloadAssetMismatch", {
        defaultValue: "更新包信息与已检查结果不一致，请重新检查更新",
      });
    case "updateDownloadClientUnavailable":
      return translate("settings.update.error.downloadClientUnavailable", {
        defaultValue: "下载客户端不可用，请稍后重试",
      });
    case "updateDownloadRedirectInvalid":
      return translate("settings.update.error.downloadRedirectInvalid", {
        defaultValue: "下载重定向地址无效",
      });
    case "updateDownloadRedirectLoop":
      return translate("settings.update.error.downloadRedirectLoop", {
        defaultValue: "下载重定向次数过多，请稍后重试",
      });
    case "updateDownloadTimeout":
      return translate("settings.update.error.downloadTimeout", {
        defaultValue: "下载请求超时，请稍后重试",
      });
    case "updateDownloadNetwork":
      return translate("settings.update.error.downloadNetwork", {
        defaultValue: "下载过程中网络中断，请稍后重试",
      });
    case "updateMirrorChyanDownloadUnavailable":
      return translate("settings.update.error.mirrorChyanDownloadUnavailable", {
        defaultValue: "MirrorChyan 下载源尚未配置，当前阶段请改用 GitHub 下载",
      });
    case "updateMirrorChyanApi":
      return translate("settings.update.error.mirrorChyanApi", {
        defaultValue: message ?? "Mirror酱 API 请求失败",
      });
    case "updateMirrorChyanCdk":
      return translate("settings.update.error.mirrorChyanCdk", {
        defaultValue: message ?? "Mirror酱 CDK 验证失败，请检查 CDK 是否正确",
      });
    case "updateMirrorChyanCdkExpired":
      return translate("settings.update.error.mirrorChyanCdkExpired", {
        defaultValue: "Mirror酱 CDK 已过期，请续费或更换",
      });
    case "updateMirrorChyanCdkInvalid":
      return translate("settings.update.error.mirrorChyanCdkInvalid", {
        defaultValue: "Mirror酱 CDK 错误，请确认输入是否正确",
      });
    case "updateMirrorChyanCdkQuotaExhausted":
      return translate("settings.update.error.mirrorChyanCdkQuotaExhausted", {
        defaultValue: "Mirror酱 CDK 今日下载次数已达上限，请明日再试或升级套餐",
      });
    case "updateMirrorChyanCdkMismatched":
      return translate("settings.update.error.mirrorChyanCdkMismatched", {
        defaultValue: "Mirror酱 CDK 类型与本应用不匹配，请确认 CDK 对应的资源",
      });
    case "updateMirrorChyanCdkBlocked":
      return translate("settings.update.error.mirrorChyanCdkBlocked", {
        defaultValue: "Mirror酱 CDK 已被封禁，请联系 Mirror酱客服处理",
      });
    case "updateMirrorChyanInvalidParams":
      return translate("settings.update.error.mirrorChyanInvalidParams", {
        defaultValue: "Mirror酱请求参数不正确，请更新应用后重试",
      });
    case "updateMirrorChyanResourceNotFound":
      return translate("settings.update.error.mirrorChyanResourceNotFound", {
        defaultValue: "Mirror酱暂无当前平台的资源，请改用 GitHub 下载",
      });
    case "updateMirrorChyanInvalidOs":
      return translate("settings.update.error.mirrorChyanInvalidOs", {
        defaultValue: "Mirror酱不支持当前操作系统参数，请更新应用后重试",
      });
    case "updateMirrorChyanInvalidArch":
      return translate("settings.update.error.mirrorChyanInvalidArch", {
        defaultValue: "Mirror酱不支持当前架构参数，请更新应用后重试",
      });
    case "updateMirrorChyanInvalidChannel":
      return translate("settings.update.error.mirrorChyanInvalidChannel", {
        defaultValue: "Mirror酱不支持当前更新通道参数，请更新应用后重试",
      });
    case "updateMirrorChyanBusiness":
      return translate("settings.update.error.mirrorChyanBusiness", {
        defaultValue: message ?? "Mirror酱返回了业务错误，请稍后重试",
      });
    case "updateMirrorChyanDownloadNeedCdk":
      return translate("settings.update.error.mirrorChyanDownloadNeedCdk", {
        defaultValue: "Mirror酱未返回下载链接，请配置有效的 CDK 后重试",
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
    case "updateInstallInterrupted":
      return translate("settings.update.error.installInterrupted", {
        defaultValue: "上次安装未完成，请重试安装",
      });
    case "updateInstallCleanupFailed":
      return translate("settings.update.error.installCleanupFailed", {
        defaultValue: "安装后清理临时文件失败",
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
