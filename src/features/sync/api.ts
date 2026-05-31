import { invoke } from "@tauri-apps/api/core";
import type { SyncStatus } from "./types";

type RawSyncStatus = Omit<SyncStatus, "lastRevision"> & {
  lastRevision: string | number;
};

function normalizeSyncStatus(status: RawSyncStatus): SyncStatus {
  return {
    ...status,
    lastRevision: String(status.lastRevision),
  };
}

export async function getSyncStatus(): Promise<SyncStatus> {
  return normalizeSyncStatus(await invoke<RawSyncStatus>("sync_status"));
}

export async function syncNow(): Promise<SyncStatus> {
  return normalizeSyncStatus(await invoke<RawSyncStatus>("sync_now"));
}

export async function testSyncConnection(): Promise<SyncStatus> {
  return normalizeSyncStatus(await invoke<RawSyncStatus>("sync_test_connection"));
}
