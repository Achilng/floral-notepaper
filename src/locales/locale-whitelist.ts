export const SUPPORTED_LOCALES = ["zh-CN", "en-US", "zh-HK"] as const;

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const DEFAULT_LOCALE: SupportedLocale = "zh-CN";

const SUPPORTED_LOCALE_SET = new Set<string>(SUPPORTED_LOCALES);

const LOCALE_ALIASES: Record<string, SupportedLocale> = {
  en: "en-US",
  "en-us": "en-US",
  zh: "zh-CN",
  "zh-cn": "zh-CN",
  "zh-hans": "zh-CN",
  "zh-sg": "zh-CN",
  "zh-hant": "zh-HK",
  "zh-hk": "zh-HK",
  "zh-mo": "zh-HK",
  "zh-tw": "zh-HK",
};

function canonicalizeLocale(locale: string): string {
  try {
    return Intl.getCanonicalLocales(locale)[0] ?? locale;
  } catch {
    return locale;
  }
}

export function normalizeLocale(locale?: string | null): SupportedLocale | null {
  if (!locale) {
    return null;
  }

  const trimmed = locale.trim();
  if (!trimmed) {
    return null;
  }

  const canonical = canonicalizeLocale(trimmed);
  if (SUPPORTED_LOCALE_SET.has(canonical)) {
    return canonical as SupportedLocale;
  }

  const segments = canonical.toLowerCase().split("-");
  for (let length = segments.length; length > 0; length -= 1) {
    const candidate = segments.slice(0, length).join("-");
    const normalized = LOCALE_ALIASES[candidate];
    if (normalized) {
      return normalized;
    }
  }

  return null;
}

export function resolveAppLocale(
  preferredLocale?: string | null,
  navigatorLocale?: string | null,
): SupportedLocale {
  return normalizeLocale(preferredLocale) ?? normalizeLocale(navigatorLocale) ?? DEFAULT_LOCALE;
}
