import { useState, useCallback } from "react";
import en from "../locales/en.json";
import es from "../locales/es.json";

type Locale = "en" | "es";

const translations: Record<Locale, Record<string, string>> = { en, es };

const STORAGE_KEY = "windows-island-locale";

function getInitialLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "en" || stored === "es") return stored;
  return "en";
}

export function useI18n() {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);

  const setLocale = useCallback(async (next: Locale) => {
    localStorage.setItem(STORAGE_KEY, next);
    setLocaleState(next);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("set_locale", { locale: next });
    } catch {
      /* browser preview — no Tauri runtime available */
    }
  }, []);

  const t = useCallback(
    (key: string): string => {
      return translations[locale][key] ?? translations["en"][key] ?? key;
    },
    [locale],
  );

  return { t, locale, setLocale };
}
