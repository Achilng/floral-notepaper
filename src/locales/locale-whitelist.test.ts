import { describe, expect, test } from "vitest";
import {
  LOCALE_OPTIONS,
  SUPPORTED_LOCALES,
  normalizeLocale,
  resolveAppLocale,
} from "./locale-whitelist";

describe("locale whitelist", () => {
  test("normalizes supported locales and known aliases", () => {
    expect(normalizeLocale("zh-CN")).toBe("zh-CN");
    expect(normalizeLocale("zh-cn")).toBe("zh-CN");
    expect(normalizeLocale("zh-TW")).toBe("zh-HK");
    expect(normalizeLocale("en-GB")).toBe("en-US");
  });

  test("keeps display metadata in sync with supported locales", () => {
    expect(LOCALE_OPTIONS.map((option) => option.value)).toEqual([...SUPPORTED_LOCALES]);
    for (const option of LOCALE_OPTIONS) {
      expect(option.labelKey).toMatch(/^settings\.locale\./);
      expect(option.defaultLabel.length).toBeGreaterThan(0);
    }
  });

  test("returns null for unsupported locales", () => {
    expect(normalizeLocale("fr-FR")).toBeNull();
    expect(normalizeLocale("")).toBeNull();
    expect(normalizeLocale(undefined)).toBeNull();
  });

  test("resolves preferred locale before browser locale and fallback", () => {
    expect(resolveAppLocale("en-US", "zh-CN")).toBe("en-US");
    expect(resolveAppLocale(undefined, "zh-HK")).toBe("zh-HK");
    expect(resolveAppLocale(undefined, "fr-FR")).toBe("zh-CN");
  });
});
