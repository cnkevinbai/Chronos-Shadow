import { describe, it, expect } from "vitest";
import { locales, getLocale } from "./i18n";

describe("i18n", () => {
  it("getLocale 返回正确语言包", () => {
    expect(getLocale("zh")).toBe(locales.zh);
    expect(getLocale("en")).toBe(locales.en);
  });

  it("未知语言回退到英文", () => {
    expect(getLocale("xx" as never)).toBe(locales.en);
  });

  it("zh 与 en 键完全一致（i18n 完整性）", () => {
    const zhKeys = Object.keys(locales.zh).sort();
    const enKeys = Object.keys(locales.en).sort();
    expect(zhKeys).toEqual(enKeys);
  });

  it("所有翻译值非空", () => {
    for (const lang of ["zh", "en"] as const) {
      const dict = locales[lang];
      for (const [key, value] of Object.entries(dict)) {
        expect(value.trim().length, `${lang}.${key} 为空`).toBeGreaterThan(0);
      }
    }
  });
});
