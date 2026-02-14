"use client";

import { useState } from "react";
import { useUserLocale } from "@/i18n/client";
import { motion, AnimatePresence } from "framer-motion";
import { Globe, Check, ChevronDown } from "lucide-react";
import { locales, localeNames, localeFlags, type Locale } from "@/i18n/config";

export function LanguageSelector({ variant = "dropdown" }: { variant?: "dropdown" | "minimal" | "flags" }) {
  const [locale, setLocale, isLoaded] = useUserLocale();
  const [isOpen, setIsOpen] = useState(false);
  const [isPending, setIsPending] = useState(false);

  const handleLocaleChange = (newLocale: Locale) => {
    if (newLocale === locale) {
      setIsOpen(false);
      return;
    }

    setIsPending(true);
    setLocale(newLocale);
  };

  if (!isLoaded) {
    return (
      <div className="w-10 h-10 rounded-xl bg-gray-800/50 animate-pulse" />
    );
  }

  // Minimal variant - just a button with current flag
  if (variant === "minimal") {
    return (
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={isPending}
        className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800/50 
          hover:bg-gray-700/50 border border-gray-700/50 text-sm text-gray-300
          transition-colors disabled:opacity-50"
      >
        <span className="text-lg">{localeFlags[locale]}</span>
        <span className="uppercase text-xs font-medium">{locale}</span>
        {isPending && (
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
            className="w-3 h-3 border-2 border-gray-500 border-t-blue-500 rounded-full"
          />
        )}
      </button>
    );
  }

  // Flags variant - horizontal list of flags
  if (variant === "flags") {
    return (
      <div className="flex items-center gap-1">
        {locales.map((l) => (
          <button
            key={l}
            onClick={() => handleLocaleChange(l)}
            disabled={isPending}
            className={`p-2 rounded-lg transition-all ${
              l === locale
                ? "bg-blue-500/20 text-blue-400 ring-1 ring-blue-500/50"
                : "hover:bg-gray-800/50 text-gray-500 hover:text-gray-300"
            } disabled:opacity-50`}
            title={localeNames[l]}
          >
            <span className="text-lg">{localeFlags[l]}</span>
          </button>
        ))}
      </div>
    );
  }

  // Default dropdown variant
  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={isPending}
        className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800/50 
          hover:bg-gray-700/50 border border-gray-700/50 text-sm text-gray-300
          transition-colors disabled:opacity-50 min-w-[140px]"
      >
        <Globe className="w-4 h-4 text-gray-400" />
        <span className="text-lg">{localeFlags[locale]}</span>
        <span className="flex-1 text-left">{localeNames[locale]}</span>
        <ChevronDown className={`w-4 h-4 text-gray-500 transition-transform ${isOpen ? "rotate-180" : ""}`} />
        {isPending && (
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
            className="w-3 h-3 border-2 border-gray-500 border-t-blue-500 rounded-full"
          />
        )}
      </button>

      <AnimatePresence>
        {isOpen && (
          <>
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 z-40"
              onClick={() => setIsOpen(false)}
            />

            {/* Dropdown */}
            <motion.div
              initial={{ opacity: 0, y: -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -10, scale: 0.95 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full mt-2 right-0 w-56 z-50"
            >
              <div className="glass-card rounded-xl border border-gray-700/50 shadow-2xl overflow-hidden">
                <div className="p-2">
                  <div className="text-xs font-medium text-gray-500 uppercase tracking-wider px-3 py-2">
                    Language
                  </div>
                  <div className="space-y-1">
                    {locales.map((l) => (
                      <button
                        key={l}
                        onClick={() => handleLocaleChange(l)}
                        disabled={isPending}
                        className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg
                          text-sm transition-colors disabled:opacity-50
                          ${
                            l === locale
                              ? "bg-blue-500/20 text-blue-400"
                              : "hover:bg-gray-800/50 text-gray-300"
                          }`}
                      >
                        <span className="text-xl">{localeFlags[l]}</span>
                        <span className="flex-1 text-left">{localeNames[l]}</span>
                        {l === locale && <Check className="w-4 h-4" />}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="px-4 py-2 bg-gray-800/30 border-t border-gray-700/50">
                  <p className="text-xs text-gray-500">
                    {locales.length} languages available
                  </p>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}

// Inline language switcher for mobile
export function InlineLanguageSelector() {
  const [locale, setLocale, isLoaded] = useUserLocale();
  const [isPending, setIsPending] = useState(false);

  const handleLocaleChange = (newLocale: Locale) => {
    if (newLocale === locale) return;
    setIsPending(true);
    setLocale(newLocale);
  };

  if (!isLoaded) return null;

  return (
    <div className="grid grid-cols-4 gap-2 p-4">
      {locales.map((l) => (
        <button
          key={l}
          onClick={() => handleLocaleChange(l)}
          disabled={isPending}
          className={`flex flex-col items-center gap-1 p-3 rounded-xl transition-all
            ${
              l === locale
                ? "bg-blue-500/20 text-blue-400 ring-1 ring-blue-500/50"
                : "bg-gray-800/50 text-gray-400 hover:bg-gray-700/50"
            } disabled:opacity-50`}
        >
          <span className="text-2xl">{localeFlags[l]}</span>
          <span className="text-xs font-medium uppercase">{l}</span>
        </button>
      ))}
    </div>
  );
}

// Compact selector for navbar
export function CompactLanguageSelector() {
  const [locale, setLocale, isLoaded] = useUserLocale();
  const [isOpen, setIsOpen] = useState(false);
  const [isPending, setIsPending] = useState(false);

  const handleLocaleChange = (newLocale: Locale) => {
    if (newLocale === locale) {
      setIsOpen(false);
      return;
    }
    setIsPending(true);
    setLocale(newLocale);
  };

  if (!isLoaded) {
    return (
      <div className="w-10 h-10 rounded-xl bg-gray-800/50 animate-pulse" />
    );
  }

  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={isPending}
        className="w-10 h-10 rounded-xl bg-gray-800/50 hover:bg-gray-700/50 
          border border-gray-700/50 flex items-center justify-center
          transition-colors disabled:opacity-50"
        title={localeNames[locale]}
      >
        <span className="text-lg">{localeFlags[locale]}</span>
      </button>

      <AnimatePresence>
        {isOpen && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 z-40"
              onClick={() => setIsOpen(false)}
            />
            <motion.div
              initial={{ opacity: 0, y: -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -10, scale: 0.95 }}
              className="absolute top-full mt-2 right-0 z-50"
            >
              <div className="glass-card rounded-xl border border-gray-700/50 shadow-2xl p-2">
                <div className="grid grid-cols-2 gap-1">
                  {locales.map((l) => (
                    <button
                      key={l}
                      onClick={() => handleLocaleChange(l)}
                      disabled={isPending}
                      className={`flex items-center gap-2 px-3 py-2 rounded-lg
                        text-sm transition-colors disabled:opacity-50
                        ${
                          l === locale
                            ? "bg-blue-500/20 text-blue-400"
                            : "hover:bg-gray-800/50 text-gray-300"
                        }`}
                    >
                      <span className="text-lg">{localeFlags[l]}</span>
                      <span className="uppercase text-xs font-medium">{l}</span>
                    </button>
                  ))}
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}
