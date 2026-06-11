import type { TFunction } from "i18next";
import { getUpdateErrorMessage } from "./updateErrors";
import type { UpdateState } from "./types";

export const ABOUT_UPDATE_LABEL_DURATION_MS = 30_000;

export type UpdateNoticeTone = "idle" | "success" | "error";

export interface UpdateInlineNotice {
  tone: UpdateNoticeTone;
  text: string;
}

export interface AboutUpdateReminderState {
  hasPendingUpdate: boolean;
  showText: boolean;
}

export function hasPendingUpdate(
  status: Pick<UpdateState, "latestVersion"> | null | undefined,
): boolean {
  return Boolean(status?.latestVersion);
}

export function createAboutUpdateReminderState(
  status: Pick<UpdateState, "latestVersion"> | null | undefined,
): AboutUpdateReminderState {
  return {
    hasPendingUpdate: hasPendingUpdate(status),
    showText: false,
  };
}

export function applyAboutUpdateStatus(
  current: AboutUpdateReminderState,
  status: Pick<UpdateState, "latestVersion"> | null | undefined,
): AboutUpdateReminderState {
  if (!hasPendingUpdate(status)) {
    return { hasPendingUpdate: false, showText: false };
  }

  if (!current.hasPendingUpdate) {
    return { hasPendingUpdate: true, showText: true };
  }

  return {
    hasPendingUpdate: true,
    showText: current.showText,
  };
}

export function dismissAboutUpdateReminderText(
  current: AboutUpdateReminderState,
): AboutUpdateReminderState {
  return {
    hasPendingUpdate: current.hasPendingUpdate,
    showText: false,
  };
}

export function getUpdateCheckCompletionNotice(
  status: UpdateState,
  translate: TFunction,
): UpdateInlineNotice | null {
  if (status.status === "available") {
    return {
      tone: "success",
      text: translate("settings.update.available", {
        version: status.latestVersion,
        defaultValue: "发现新版本 {{version}}",
      }),
    };
  }

  if (status.status === "idle" && status.checkedAt) {
    return {
      tone: "success",
      text: translate("settings.update.notAvailable", {
        defaultValue: "当前已是最新版本",
      }),
    };
  }

  if (status.status === "failed" && status.lastError) {
    return {
      tone: "error",
      text: getUpdateErrorMessage(status.lastError, translate),
    };
  }

  return null;
}

export function getInitialUpdateStatusNotice(
  status: UpdateState | null | undefined,
  translate: TFunction,
): UpdateInlineNotice | null {
  if (status?.status === "failed" && status.lastError) {
    return {
      tone: "error",
      text: getUpdateErrorMessage(status.lastError, translate),
    };
  }

  return null;
}
