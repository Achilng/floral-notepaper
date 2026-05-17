import type { Language, TranslationMap } from "../types";
import { zhCN } from "./zh-CN";
import { en } from "./en";
import { zhTW } from "./zh-TW";

export const translations: Record<Language, TranslationMap> = {
  "zh-CN": zhCN,
  en,
  "zh-TW": zhTW,
};

export const languageLabels: Array<{ value: Language; label: string }> = [
  { value: "zh-CN", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "en", label: "English" },
];
