// Chronos-Shadow i18n Context
import { createContext, useContext, useState, useCallback, type ReactNode } from "react";
import { type Lang, type LocaleDict, getLocale } from "./i18n";

interface I18nContextType {
  lang: Lang;
  t: LocaleDict;
  setLang: (lang: Lang) => void;
}

const I18nContext = createContext<I18nContextType>({
  lang: "zh",
  t: getLocale("zh"),
  setLang: () => {},
});

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(() => {
    try {
      const saved = localStorage.getItem("cs-lang");
      if (saved === "en" || saved === "zh") return saved;
    } catch {}
    // 国际版：按浏览器/系统语言自动探测（英文环境 → en，其余 → zh）
    try {
      if (typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("en")) {
        return "en";
      }
    } catch {}
    return "zh";
  });

  const setLang = useCallback((l: Lang) => {
    setLangState(l);
    try { localStorage.setItem("cs-lang", l); } catch {}
  }, []);

  return (
    <I18nContext.Provider value={{ lang, t: getLocale(lang), setLang }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useT(): LocaleDict {
  return useContext(I18nContext).t;
}

export function useLang() {
  const ctx = useContext(I18nContext);
  return { lang: ctx.lang, setLang: ctx.setLang };
}
