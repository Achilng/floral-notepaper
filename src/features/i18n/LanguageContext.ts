import { createContext, useContext } from "react";
import type { Language, TranslationKey } from "./types";
import { translations } from "./translations/index";

interface LanguageContextValue {
  language: Language;
  setLanguage: (lang: Language) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

export const LanguageContext = createContext<LanguageContextValue>({
  language: "zh-CN",
  setLanguage: () => {},
  t: (key) => String(key),
});

export function useTranslation() {
  return useContext(LanguageContext);
}

export function translate(
  language: Language,
  key: TranslationKey,
  params?: Record<string, string | number>,
): string {
  const map = translations[language] ?? translations["zh-CN"];
  let text = map[key] ?? String(key);

  if (params) {
    for (const [paramKey, paramValue] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${paramKey}\\}`, "g"), String(paramValue));
    }
  }

  return text;
}
