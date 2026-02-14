"use client";

import { useEffect, useState } from "react";
import { Locale, defaultLocale } from "./config";

const COOKIE_NAME = "NEXT_LOCALE";

export function useUserLocale(): [Locale, (locale: Locale) => void, boolean] {
  const [locale, setLocaleState] = useState<Locale>(defaultLocale);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    // Get locale from localStorage or cookie on client
    const stored = localStorage.getItem(COOKIE_NAME) || 
      document.cookie.match(new RegExp(`${COOKIE_NAME}=([^;]+)`))?.[1];
    
    if (stored && isValidLocale(stored)) {
      setLocaleState(stored as Locale);
    }
    setIsLoaded(true);
  }, []);

  const setLocale = (newLocale: Locale) => {
    setLocaleState(newLocale);
    localStorage.setItem(COOKIE_NAME, newLocale);
    document.cookie = `${COOKIE_NAME}=${newLocale};path=/;max-age=31536000`;
    window.location.reload();
  };

  return [locale, setLocale, isLoaded];
}

function isValidLocale(locale: string): locale is Locale {
  return ["en", "bg", "de", "es", "fr", "it", "ru"].includes(locale);
}
