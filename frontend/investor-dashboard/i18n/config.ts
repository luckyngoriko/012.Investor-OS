export type Locale = (typeof locales)[number];

export const locales = [
  "en", // English
  "bg", // Bulgarian
  "de", // German
  "es", // Spanish
  "fr", // French
  "it", // Italian
  "ru", // Russian
] as const;

export const defaultLocale: Locale = "bg";

export const localeNames: Record<Locale, string> = {
  en: "English",
  bg: "Български",
  de: "Deutsch",
  es: "Español",
  fr: "Français",
  it: "Italiano",
  ru: "Русский",
};

export const localeFlags: Record<Locale, string> = {
  en: "🇬🇧",
  bg: "🇧🇬",
  de: "🇩🇪",
  es: "🇪🇸",
  fr: "🇫🇷",
  it: "🇮🇹",
  ru: "🇷🇺",
};

// RTL languages (for future use)
export const rtlLocales: Locale[] = [];

export function isRTL(locale: Locale): boolean {
  return rtlLocales.includes(locale);
}
