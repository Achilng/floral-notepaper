/**
 * Synchronous platform detection utilities.
 *
 * Uses navigator.platform for instant (no-flash) checks;
 * falls back to the Tauri platform_os command for authoritative results.
 */

let _platformCache: string | null = null;
let _linuxWindowEnvironmentCache: LinuxWindowEnvironment | null = null;

export interface LinuxWindowEnvironment {
  os: string;
  xdgCurrentDesktop: string | null;
  desktopSession: string | null;
  xSessionDesktop: string | null;
  waylandDisplay: string | null;
  display: string | null;
  i3sock: string | null;
  swaysock: string | null;
  hyprlandInstanceSignature: string | null;
  niriSocket: string | null;
  isTilingWm: boolean;
}

/** Synchronous check — safe to call during render. */
export function isWindowsSync(): boolean {
  return (
    typeof navigator !== "undefined" && /win/i.test(navigator.platform)
  );
}

/** Synchronous check — safe to call during render. */
export function isLinuxSync(): boolean {
  return (
    typeof navigator !== "undefined" && /linux/i.test(navigator.platform)
  );
}

/**
 * Authoritative platform string via Tauri command.
 * Prefer the sync helpers for render-time decisions; use this for logging or
 * fine-grained branching.
 */
export async function getPlatformOs(): Promise<string> {
  if (_platformCache) return _platformCache;
  const { invoke } = await import("@tauri-apps/api/core");
  _platformCache = await invoke<string>("platform_os");
  return _platformCache;
}

export async function getLinuxWindowEnvironment(): Promise<LinuxWindowEnvironment | null> {
  if (!isLinuxSync()) return null;
  if (_linuxWindowEnvironmentCache) return _linuxWindowEnvironmentCache;

  const { invoke } = await import("@tauri-apps/api/core");
  _linuxWindowEnvironmentCache = await invoke<LinuxWindowEnvironment>(
    "linux_window_environment",
  );
  return _linuxWindowEnvironmentCache;
}

export async function isLinuxTilingWindowManager(): Promise<boolean> {
  const environment = await getLinuxWindowEnvironment();
  return environment?.isTilingWm ?? false;
}
