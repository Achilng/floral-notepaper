export type Language = "zh-CN" | "en" | "zh-TW";

export type TranslationKey = keyof typeof import("./translations/zh-CN").zhCN;

export interface TranslationMap {
  [key: string]: string;
}
