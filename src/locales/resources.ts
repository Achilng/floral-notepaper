import enUS from "./en-US/translation.json";
import zhCN from "./zh-CN/translation.json";
import zhHK from "./zh-HK/translation.json";

export interface TranslationTree {
  [key: string]: string | TranslationTree;
}

function isTranslationTree(value: string | TranslationTree | undefined): value is TranslationTree {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function mergeTranslations(source: TranslationTree, overrides: TranslationTree): TranslationTree {
  const merged: TranslationTree = { ...source };

  for (const [key, value] of Object.entries(overrides)) {
    const sourceValue = merged[key];

    if (isTranslationTree(sourceValue) && isTranslationTree(value)) {
      merged[key] = mergeTranslations(sourceValue, value);
      continue;
    }

    merged[key] = value;
  }

  return merged;
}

export const translationOverrides = {
  "zh-CN": zhCN,
  "en-US": enUS,
  "zh-HK": zhHK,
} as const satisfies Record<string, TranslationTree>;

export const resolvedTranslations = {
  "zh-CN": translationOverrides["zh-CN"],
  "en-US": mergeTranslations(translationOverrides["zh-CN"], translationOverrides["en-US"]),
  "zh-HK": mergeTranslations(translationOverrides["zh-CN"], translationOverrides["zh-HK"]),
} as const;

export const resources = {
  "zh-CN": { translation: resolvedTranslations["zh-CN"] },
  "en-US": { translation: resolvedTranslations["en-US"] },
  "zh-HK": { translation: resolvedTranslations["zh-HK"] },
} as const;
