"use client";

import { createContext, useContext, useEffect, useState, ReactNode } from "react";
import { Locale, defaultLocale, locales } from "@/i18n/config";

type Messages = Record<string, string | Record<string, string>>;

interface I18nContextType {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: string, params?: Record<string, string>) => string;
  isLoading: boolean;
}

const I18nContext = createContext<I18nContextType | null>(null);

const COOKIE_NAME = "NEXT_LOCALE";

function isValidLocale(locale: string): locale is Locale {
  return locales.includes(locale as Locale);
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(defaultLocale);
  const [messages, setMessages] = useState<Messages>({});
  const [isLoading, setIsLoading] = useState(true);

  // Load locale from storage on mount
  useEffect(() => {
    const stored = localStorage.getItem(COOKIE_NAME);
    if (stored && isValidLocale(stored)) {
      setLocaleState(stored);
    }
  }, []);

  // Load messages when locale changes
  useEffect(() => {
    setIsLoading(true);
    
    // Dynamic import of messages
    import(`@/messages/${locale}.json`)
      .then((module) => {
        setMessages(module.default);
        setIsLoading(false);
      })
      .catch((err) => {
        console.error(`Failed to load messages for ${locale}:`, err);
        // Fallback to default locale
        if (locale !== defaultLocale) {
          setLocaleState(defaultLocale);
        }
        setIsLoading(false);
      });
  }, [locale]);

  const setLocale = (newLocale: Locale) => {
    setLocaleState(newLocale);
    localStorage.setItem(COOKIE_NAME, newLocale);
    document.cookie = `${COOKIE_NAME}=${newLocale};path=/;max-age=31536000`;
    // Reload to apply new messages
    window.location.reload();
  };

  const t = (key: string, params?: Record<string, string>): string => {
    const keys = key.split(".");
    let value: unknown = messages;

    for (const k of keys) {
      if (typeof value === "object" && value !== null && k in value) {
        value = (value as Record<string, unknown>)[k];
      } else {
        return key; // Return key as fallback
      }
    }

    if (typeof value !== "string") {
      return key;
    }

    // Replace params
    if (params) {
      return Object.entries(params).reduce(
        (acc, [key, val]) => acc.replace(`{${key}}`, val),
        value
      );
    }

    return value;
  };

  return (
    <I18nContext.Provider value={{ locale, setLocale, t, isLoading }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return context;
}

// Hook for translations
export function useTranslations(namespace?: string) {
  const { t, isLoading } = useI18n();

  return {
    t: (key: string, params?: Record<string, string>) => {
      const fullKey = namespace ? `${namespace}.${key}` : key;
      return t(fullKey, params);
    },
    isLoading,
  };
}
