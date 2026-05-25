import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { DEFAULT_LOCALE, resolveAppLocale, SUPPORTED_LOCALES } from "./locale-whitelist";
import { resources } from "./resources";

let initPromise: Promise<typeof i18n> | null = null;

function browserLocale(): string | undefined {
  if (typeof navigator === "undefined") {
    return undefined;
  }

  return navigator.language;
}

export function initializeI18n(preferredLocale?: string | null) {
  if (!initPromise) {
    initPromise = i18n
      .use(initReactI18next)
      .init({
        resources,
        lng: resolveAppLocale(preferredLocale, browserLocale()),
        fallbackLng: DEFAULT_LOCALE,
        supportedLngs: [...SUPPORTED_LOCALES],
        defaultNS: "translation",
        ns: ["translation"],
        interpolation: {
          escapeValue: false,
        },
        returnEmptyString: false,
        returnNull: false,
      })
      .then(() => i18n);
  }

  return initPromise;
}

export async function syncLanguage(locale?: string | null) {
  const nextLocale = resolveAppLocale(locale, browserLocale());

  if (!i18n.isInitialized) {
    await initializeI18n(nextLocale);
    return i18n;
  }

  if (i18n.resolvedLanguage !== nextLocale && i18n.language !== nextLocale) {
    await i18n.changeLanguage(nextLocale);
  }

  return i18n;
}

export { i18n };

export default i18n;
