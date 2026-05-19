import { useCallback, useEffect, useState } from "react";
import { checkForUpdate, openReleasePage, type UpdateInfo } from "./api";

const CHECK_INTERVAL = 6 * 60 * 60 * 1000;

export function useAutoUpdate() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [dismissed, setDismissed] = useState(false);

  const doCheck = useCallback(async () => {
    const info = await checkForUpdate();
    if (info) {
      setUpdateInfo(info);
      setDismissed(false);
    }
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void doCheck();
    }, 5000);
    const interval = window.setInterval(() => {
      void doCheck();
    }, CHECK_INTERVAL);
    return () => {
      window.clearTimeout(timer);
      window.clearInterval(interval);
    };
  }, [doCheck]);

  const dismiss = useCallback(() => {
    setDismissed(true);
  }, []);

  const openDownload = useCallback(() => {
    if (updateInfo) {
      void openReleasePage(updateInfo.url);
    }
  }, [updateInfo]);

  return {
    updateInfo: dismissed ? null : updateInfo,
    dismiss,
    openDownload,
    checkNow: doCheck,
  };
}
