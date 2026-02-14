# Investor OS - Internationalization (i18n) Guide

## 🌍 Поддържани Езици

Investor OS поддържа 7 езика:

| Език | Код | Флаг | Име |
|------|-----|------|-----|
| 🇧🇬 Български | `bg` | 🇧🇬 | Български |
| 🇬🇧 English | `en` | 🇬🇧 | English |
| 🇩🇪 Deutsch | `de` | 🇩🇪 | Deutsch |
| 🇪🇸 Español | `es` | 🇪🇸 | Español |
| 🇫🇷 Français | `fr` | 🇫🇷 | Français |
| 🇮🇹 Italiano | `it` | 🇮🇹 | Italiano |
| 🇷🇺 Русский | `ru` | 🇷🇺 | Русский |

---

## 📁 Структура на Файловете

```
frontend/investor-dashboard/
├── i18n/
│   ├── config.ts          # Конфигурация на езиците
│   ├── client.ts          # Client-side locale management
│   └── user-locale.ts     # Server-side locale (for future SSR)
├── messages/
│   ├── bg.json            # Български
│   ├── en.json            # English
│   ├── de.json            # Deutsch
│   ├── es.json            # Español
│   ├── fr.json            # Français
│   ├── it.json            # Italiano
│   └── ru.json            # Русский
└── components/
    ├── i18n-provider.tsx  # React Context Provider
    └── language-selector.tsx  # UI Components
```

---

## 🚀 Използване

### 1. В Компоненти

```tsx
import { useTranslations } from "@/components/i18n-provider";

function MyComponent() {
  const { t } = useTranslations("navigation");
  
  return <h1>{t("dashboard")}</h1>; // "Табло" или "Dashboard"
}
```

### 2. С Параметри

```tsx
const { t } = useTranslations("validation");

t("minValue", { min: "10" }); // "Минимална стойност: 10"
```

### 3. Language Selector

```tsx
import { LanguageSelector, CompactLanguageSelector } from "@/components/language-selector";

// Пълен dropdown
<LanguageSelector />

// Компактен за navbar
<CompactLanguageSelector />

// Само флагове
<LanguageSelector variant="flags" />
```

---

## 📝 Структура на Преводите

### Основни Секции

```json
{
  "metadata": {
    "title": "...",
    "description": "..."
  },
  "navigation": {
    "dashboard": "...",
    "portfolio": "...",
    // ... други
  },
  "navGroups": {
    "main": "...",
    "aiTrading": "...",
    "management": "...",
    "system": "..."
  },
  "navDescriptions": {
    "dashboard": "...",
    // ... описания
  },
  "auth": {
    "login": "...",
    "logout": "..."
  },
  "dashboard": {
    "title": "...",
    "portfolioValue": "..."
  },
  "trading": {
    "mode": "...",
    "manual": "...",
    // ...
  },
  "common": {
    "save": "...",
    "cancel": "..."
  },
  "validation": {
    "required": "..."
  }
}
```

---

## 🔄 Добавяне на Нов Език

1. **Създайте JSON файл** в `messages/xx.json`

2. **Добавете в конфигурацията** (`i18n/config.ts`):
```typescript
export const locales = [
  "en",
  "bg",
  // ... други
  "xx",  // нов език
] as const;

export const localeNames: Record<Locale, string> = {
  // ... други
  xx: "Language Name",
};

export const localeFlags: Record<Locale, string> = {
  // ... други
  xx: "🇽🇽",
};
```

3. **Копирайте и преведете** съдържанието от `en.json`

---

## 🎯 RTL Поддръжка

За езици от дясно на ляво (Arabic, Hebrew):

```typescript
// i18n/config.ts
export const rtlLocales: Locale[] = ["ar", "he"];

export function isRTL(locale: Locale): boolean {
  return rtlLocales.includes(locale);
}
```

В layout:
```tsx
<html lang={locale} dir={isRTL(locale) ? "rtl" : "ltr"}>
```

---

## 💾 Съхранение на Избор

Езикът се съхранява в:
- **localStorage** - за персистенция между сесиите
- **Cookie** - за бъдеща SSR поддръжка

```typescript
const COOKIE_NAME = "NEXT_LOCALE";

// Зареждане
const stored = localStorage.getItem(COOKIE_NAME);

// Запазване
localStorage.setItem(COOKIE_NAME, locale);
document.cookie = `${COOKIE_NAME}=${locale};path=/;max-age=31536000`;
```

---

## 🧪 Тестване

```bash
# Build проверка
cd frontend/investor-dashboard
npm run build

# Проверка на TypeScript
npx tsc --noEmit
```

---

## 📊 Покритие на Преводите

| Секция | BG | EN | DE | ES | FR | IT | RU |
|--------|----|----|----|----|----|----|----|
| Navigation | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Auth | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Dashboard | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Trading | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Common | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Validation | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 🔮 Бъдещи Подобрения

- [ ] SSR (Server-Side Rendering) поддръжка
- [ ] Автоматично откриване на езика на браузъра
- [ ] RTL езици (Arabic, Hebrew)
- [ ] Crowdin интеграция за управление на преводи
- [ ] Pluralization rules
- [ ] Date/Number форматиране според локала
