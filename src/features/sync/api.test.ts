import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { getSyncStatus, syncNow, testSyncConnection } from "./api";
import type { SyncStatus } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("sync api", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  test("gets sync status through Rust", async () => {
    const status: SyncStatus = {
      enabled: true,
      configured: true,
      lastRevision: "etag-1",
      lastSyncAt: "2026-05-22T03:00:00Z",
      lastError: null,
    };
    mockedInvoke.mockResolvedValue(status);

    await expect(getSyncStatus()).resolves.toEqual(status);

    expect(invoke).toHaveBeenCalledWith("sync_status");
  });

  test("runs manual sync through Rust", async () => {
    const status: SyncStatus = {
      enabled: true,
      configured: true,
      lastRevision: "etag-2",
      lastSyncAt: "2026-05-22T03:01:00Z",
      lastError: null,
    };
    mockedInvoke.mockResolvedValue(status);

    await expect(syncNow()).resolves.toEqual(status);

    expect(invoke).toHaveBeenCalledWith("sync_now");
  });

  test("tests the configured WebDAV connection through Rust", async () => {
    const status: SyncStatus = {
      enabled: true,
      configured: true,
      lastRevision: "etag-2",
      lastSyncAt: null,
      lastError: null,
    };
    mockedInvoke.mockResolvedValue(status);

    await expect(testSyncConnection()).resolves.toEqual(status);

    expect(invoke).toHaveBeenCalledWith("sync_test_connection");
  });
});
