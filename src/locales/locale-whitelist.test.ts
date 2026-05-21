import { describe, expect, test } from "vitest";
import { normalizeLocale, resolveAppLocale } from "./locale-whitelist";

describe("locale whitelist", () => {
  test("normalizes supported locales and known aliases", () => {
    expect(normalizeLocale("zh-CN")).toBe("zh-CN");
    expect(normalizeLocale("zh-cn")).toBe("zh-CN");
    expect(normalizeLocale("zh-TW")).toBe("zh-HK");
    expect(normalizeLocale("en-GB")).toBe("en-US");
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
