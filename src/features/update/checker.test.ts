import { describe, expect, test, vi } from "vitest";
import {
  checkForAppUpdate,
  compareVersions,
  extractLatestReleaseInfo,
  normalizeVersion,
  RELEASES_API_URL,
} from "./checker";

describe("update checker helpers", () => {
  test("normalizes release versions", () => {
    expect(normalizeVersion("v1.2.3")).toBe("1.2.3");
    expect(normalizeVersion(" 1.2.3-beta ")).toBe("1.2.3");
  });

  test("compares semantic version segments", () => {
    expect(compareVersions("1.2.0", "1.1.9")).toBe(1);
    expect(compareVersions("1.2.0", "1.2.0")).toBe(0);
    expect(compareVersions("1.2.0", "1.2.1")).toBe(-1);
    expect(compareVersions("1.2", "1.2.0")).toBe(0);
  });

  test("extracts latest release info from GitHub payload", () => {
    expect(
      extractLatestReleaseInfo({
        tag_name: "v1.0.4",
        html_url: "https://github.com/Achilng/floral-notepaper/releases/tag/v1.0.4",
      }),
    ).toEqual({
      version: "1.0.4",
      tagName: "v1.0.4",
      url: "https://github.com/Achilng/floral-notepaper/releases/tag/v1.0.4",
      assetName: null,
      assetUrl: null,
    });
  });

  test("selects the Windows installer asset when available", () => {
    expect(
      extractLatestReleaseInfo(
        {
          tag_name: "v1.0.5",
          html_url: "https://example.com/release",
          assets: [
            {
              name: "floral-notepaper_1.0.5.exe",
              browser_download_url: "https://example.com/portable.exe",
            },
            {
              name: "floral-notepaper_1.0.5_x64-setup.exe",
              browser_download_url: "https://example.com/setup.exe",
            },
          ],
        },
        "windows",
      ),
    ).toEqual({
      version: "1.0.5",
      tagName: "v1.0.5",
      url: "https://example.com/release",
      assetName: "floral-notepaper_1.0.5_x64-setup.exe",
      assetUrl: "https://example.com/setup.exe",
    });
  });
});

describe("checkForAppUpdate", () => {
  test("downloads and launches the installer when a newer version is available and confirmed", async () => {
    const fetchRelease = vi.fn(async () => ({
      ok: true,
      json: async () => ({
        tag_name: "v1.0.5",
        html_url: "https://example.com/release",
        assets: [
          {
            name: "floral-notepaper_1.0.5_x64-setup.exe",
            browser_download_url: "https://example.com/setup.exe",
          },
        ],
      }),
    })) as unknown as typeof fetch;
    const confirmOpen = vi.fn(async () => true);
    const openReleasePage = vi.fn(async () => undefined);
    const downloadAsset = vi.fn(async () => "D:\\Temp\\setup.exe");
    const launchInstaller = vi.fn(async () => undefined);
    const showMessage = vi.fn(async () => "Ok");

    await checkForAppUpdate({
      getCurrentVersion: async () => "1.0.4",
      fetchRelease,
      confirmOpen,
      showMessage,
      openReleasePage,
      downloadAsset,
      launchInstaller,
      platform: "windows",
    });

    expect(fetchRelease).toHaveBeenCalledWith(
      RELEASES_API_URL,
      expect.objectContaining({
        headers: expect.objectContaining({
          Accept: "application/vnd.github+json",
        }),
      }),
    );
    expect(confirmOpen).toHaveBeenCalledOnce();
    expect(downloadAsset).toHaveBeenCalledWith(
      "https://example.com/setup.exe",
      "floral-notepaper_1.0.5_x64-setup.exe",
    );
    expect(showMessage).toHaveBeenCalledOnce();
    expect(launchInstaller).toHaveBeenCalledWith("D:\\Temp\\setup.exe");
    expect(openReleasePage).not.toHaveBeenCalled();
  });

  test("does nothing when already up to date", async () => {
    const confirmOpen = vi.fn(async () => true);
    const openReleasePage = vi.fn(async () => undefined);

    await checkForAppUpdate({
      getCurrentVersion: async () => "1.0.4",
      fetchRelease: vi.fn(async () => ({
        ok: true,
        json: async () => ({
          tag_name: "v1.0.4",
          html_url: "https://example.com/release",
        }),
      })) as unknown as typeof fetch,
      confirmOpen,
      showMessage: vi.fn(async () => "Ok"),
      openReleasePage,
      downloadAsset: vi.fn(async () => "D:\\Temp\\setup.exe"),
      launchInstaller: vi.fn(async () => undefined),
      platform: "windows",
    });

    expect(confirmOpen).not.toHaveBeenCalled();
    expect(openReleasePage).not.toHaveBeenCalled();
  });

  test("falls back to the release page when downloading fails", async () => {
    const openReleasePage = vi.fn(async () => undefined);
    const showMessage = vi.fn(async () => "Ok");

    await checkForAppUpdate({
      getCurrentVersion: async () => "1.0.4",
      fetchRelease: vi.fn(async () => ({
        ok: true,
        json: async () => ({
          tag_name: "v1.0.5",
          html_url: "https://example.com/release",
          assets: [
            {
              name: "floral-notepaper_1.0.5_x64-setup.exe",
              browser_download_url: "https://example.com/setup.exe",
            },
          ],
        }),
      })) as unknown as typeof fetch,
      confirmOpen: vi.fn(async () => true),
      showMessage,
      openReleasePage,
      downloadAsset: vi.fn(async () => {
        throw new Error("download failed");
      }),
      launchInstaller: vi.fn(async () => undefined),
      platform: "windows",
    });

    expect(showMessage).toHaveBeenCalledOnce();
    expect(openReleasePage).toHaveBeenCalledWith("https://example.com/release");
  });

  test("does nothing when user cancels the prompt", async () => {
    const openReleasePage = vi.fn(async () => undefined);

    await checkForAppUpdate({
      getCurrentVersion: async () => "1.0.4",
      fetchRelease: vi.fn(async () => ({
        ok: true,
        json: async () => ({
          tag_name: "v1.0.5",
          html_url: "https://example.com/release",
          assets: [
            {
              name: "floral-notepaper_1.0.5_x64-setup.exe",
              browser_download_url: "https://example.com/setup.exe",
            },
          ],
        }),
      })) as unknown as typeof fetch,
      confirmOpen: vi.fn(async () => false),
      showMessage: vi.fn(async () => "Ok"),
      openReleasePage,
      downloadAsset: vi.fn(async () => "D:\\Temp\\setup.exe"),
      launchInstaller: vi.fn(async () => undefined),
      platform: "windows",
    });

    expect(openReleasePage).not.toHaveBeenCalled();
  });
});
