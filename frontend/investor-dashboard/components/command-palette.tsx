"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Search,
  X,
  Command,
  Home,
  PieChart,
  TrendingUp,
  Target,
  Shield,
  Settings,
  LogOut,
  FileText,
  Activity,
  Sparkles,
  Brain,
  BarChart3,
  Calculator,
  Server,
  Lock,
  RefreshCw,
  Zap,
  Moon,
  Sun,
  Globe,
  Bell,
  HelpCircle,
  Keyboard,
  ArrowRight,
  Clock,
  Star,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth-context";
import { useUserLocale } from "@/i18n/client";
import { localeNames, localeFlags, type Locale } from "@/i18n/config";

// Command types
interface CommandItem {
  id: string;
  title: string;
  subtitle?: string;
  icon: React.ElementType;
  shortcut?: string;
  action: () => void;
  category: string;
  keywords?: string[];
}

interface CommandGroup {
  name: string;
  commands: CommandItem[];
}

export function CommandPalette() {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const router = useRouter();
  const { logout } = useAuth();
  const [locale, setLocale] = useUserLocale();

  // Navigation commands
  const navigationCommands: CommandItem[] = useMemo(
    () => [
      {
        id: "nav-dashboard",
        title: "Dashboard",
        subtitle: "View portfolio overview",
        icon: Home,
        shortcut: "G D",
        category: "Navigation",
        action: () => router.push("/"),
      },
      {
        id: "nav-portfolio",
        title: "Portfolio",
        subtitle: "Portfolio analysis and allocation",
        icon: PieChart,
        shortcut: "G P",
        category: "Navigation",
        action: () => router.push("/portfolio"),
      },
      {
        id: "nav-positions",
        title: "Positions",
        subtitle: "View active positions",
        icon: TrendingUp,
        shortcut: "G O",
        category: "Navigation",
        action: () => router.push("/positions"),
      },
      {
        id: "nav-chart",
        title: "Trading Chart",
        subtitle: "Technical analysis",
        icon: BarChart3,
        shortcut: "G C",
        category: "Navigation",
        action: () => router.push("/chart"),
      },
      {
        id: "nav-proposals",
        title: "AI Proposals",
        subtitle: "Review AI trading ideas",
        icon: Target,
        shortcut: "G A",
        category: "Navigation",
        action: () => router.push("/proposals"),
      },
      {
        id: "nav-strategy",
        title: "Strategy Selector",
        subtitle: "Choose trading strategies",
        icon: Brain,
        category: "Navigation",
        action: () => router.push("/strategy"),
      },
      {
        id: "nav-backtest",
        title: "Backtesting",
        subtitle: "Test trading strategies",
        icon: RefreshCw,
        category: "Navigation",
        action: () => router.push("/backtest"),
      },
      {
        id: "nav-risk",
        title: "Risk Management",
        subtitle: "VaR and position limits",
        icon: Shield,
        category: "Navigation",
        action: () => router.push("/risk"),
      },
      {
        id: "nav-monitoring",
        title: "Monitoring",
        subtitle: "System metrics and alerts",
        icon: Activity,
        category: "Navigation",
        action: () => router.push("/monitoring"),
      },
      {
        id: "nav-settings",
        title: "Settings",
        subtitle: "System configuration",
        icon: Settings,
        shortcut: "G S",
        category: "Navigation",
        action: () => router.push("/settings"),
      },
    ],
    [router]
  );

  // Action commands
  const actionCommands: CommandItem[] = useMemo(
    () => [
      {
        id: "action-theme",
        title: "Toggle Theme",
        subtitle: "Switch between dark and light mode",
        icon: Sun,
        shortcut: "⌘ T",
        category: "Actions",
        action: () => {
          // Toggle theme logic
          document.documentElement.classList.toggle("dark");
        },
      },
      {
        id: "action-help",
        title: "Open Help",
        subtitle: "Show help panel",
        icon: HelpCircle,
        shortcut: "?",
        category: "Actions",
        action: () => {
          // Trigger help panel
          window.dispatchEvent(new CustomEvent("toggleHelp"));
        },
      },
      {
        id: "action-notifications",
        title: "Notifications",
        subtitle: "View recent notifications",
        icon: Bell,
        shortcut: "⌘ N",
        category: "Actions",
        action: () => {
          // Open notifications
        },
      },
      {
        id: "action-refresh",
        title: "Refresh Data",
        subtitle: "Reload all dashboard data",
        icon: RefreshCw,
        shortcut: "⌘ R",
        category: "Actions",
        action: () => {
          window.location.reload();
        },
      },
      {
        id: "action-logout",
        title: "Logout",
        subtitle: "Sign out of your account",
        icon: LogOut,
        category: "Actions",
        action: () => logout(),
      },
    ],
    [logout]
  );

  // Language commands
  const languageCommands: CommandItem[] = useMemo(
    () =>
      ["en", "bg", "de", "es", "fr", "it", "ru"].map((lang) => ({
        id: `lang-${lang}`,
        title: localeNames[lang as Locale],
        subtitle: `Switch to ${localeNames[lang as Locale]}`,
        icon: () => <span className="text-lg">{localeFlags[lang as Locale]}</span>,
        category: "Language",
        action: () => {
          setLocale(lang as Locale);
        },
      })),
    [locale, setLocale]
  );

  // All commands
  const allCommands = useMemo(
    () => [...navigationCommands, ...actionCommands, ...languageCommands],
    [navigationCommands, actionCommands, languageCommands]
  );

  // Filter commands based on search
  const filteredCommands = useMemo(() => {
    if (!search.trim()) return allCommands;
    const query = search.toLowerCase();
    return allCommands.filter(
      (cmd) =>
        cmd.title.toLowerCase().includes(query) ||
        cmd.subtitle?.toLowerCase().includes(query) ||
        cmd.category.toLowerCase().includes(query) ||
        cmd.keywords?.some((k) => k.toLowerCase().includes(query))
    );
  }, [search, allCommands]);

  // Group filtered commands
  const groupedCommands = useMemo(() => {
    const groups: Record<string, CommandItem[]> = {};
    filteredCommands.forEach((cmd) => {
      if (!groups[cmd.category]) groups[cmd.category] = [];
      groups[cmd.category].push(cmd);
    });
    return Object.entries(groups).map(([name, commands]) => ({
      name,
      commands,
    }));
  }, [filteredCommands]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K to open
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setIsOpen(true);
      }
      // ESC to close
      if (e.key === "Escape" && isOpen) {
        setIsOpen(false);
      }
      // ? to open (when not in input)
      if (e.key === "?" && !isOpen && !(e.target instanceof HTMLInputElement)) {
        e.preventDefault();
        setIsOpen(true);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  // Handle arrow navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const totalCommands = filteredCommands.length;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) => (prev + 1) % totalCommands);
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) => (prev - 1 + totalCommands) % totalCommands);
          break;
        case "Enter":
          e.preventDefault();
          const selected = filteredCommands[selectedIndex];
          if (selected) {
            selected.action();
            setIsOpen(false);
          }
          break;
      }
    },
    [filteredCommands, selectedIndex]
  );

  // Reset selection when search changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [search]);

  if (!isOpen) {
    return (
      <button
        onClick={() => setIsOpen(true)}
        className="hidden lg:flex items-center gap-2 px-3 py-2 rounded-lg
          bg-gray-800/50 hover:bg-gray-700/50 border border-gray-700/50
          text-sm text-gray-400 hover:text-gray-200 transition-all"
      >
        <Search className="w-4 h-4" />
        <span className="text-sm">Search...</span>
        <kbd className="ml-2 px-1.5 py-0.5 text-xs bg-gray-700 rounded">⌘K</kbd>
      </button>
    );
  }

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm"
            onClick={() => setIsOpen(false)}
          />

          {/* Command Palette */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: -20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: -20 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-x-4 top-[20%] md:inset-x-auto md:left-1/2 md:-translate-x-1/2 
              md:w-[600px] max-w-2xl z-50"
            onKeyDown={handleKeyDown}
          >
            <div className="glass-card rounded-2xl border border-gray-700/50 shadow-2xl overflow-hidden">
              {/* Search Header */}
              <div className="flex items-center gap-3 px-4 py-4 border-b border-gray-800/50">
                <Search className="w-5 h-5 text-gray-400" />
                <input
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search commands, pages, or actions..."
                  className="flex-1 bg-transparent text-white placeholder-gray-500 
                    outline-none text-base"
                  autoFocus
                />
                <kbd
                  className="px-2 py-1 text-xs bg-gray-700 rounded cursor-pointer"
                  onClick={() => setIsOpen(false)}
                >
                  ESC
                </kbd>
              </div>

              {/* Results */}
              <div className="max-h-[400px] overflow-y-auto py-2">
                {groupedCommands.length === 0 ? (
                  <div className="px-4 py-8 text-center text-gray-500">
                    <Command className="w-12 h-12 mx-auto mb-3 opacity-30" />
                    <p>No commands found</p>
                    <p className="text-sm mt-1">
                      Try searching for something else
                    </p>
                  </div>
                ) : (
                  groupedCommands.map((group, groupIndex) => (
                    <div key={group.name}>
                      <div className="px-4 py-2 text-xs font-medium text-gray-500 uppercase tracking-wider">
                        {group.name}
                      </div>
                      {group.commands.map((command, cmdIndex) => {
                        const globalIndex =
                          groupedCommands
                            .slice(0, groupIndex)
                            .reduce((acc, g) => acc + g.commands.length, 0) +
                          cmdIndex;
                        const isSelected = globalIndex === selectedIndex;

                        return (
                          <button
                            key={command.id}
                            onClick={() => {
                              command.action();
                              setIsOpen(false);
                            }}
                            onMouseEnter={() => setSelectedIndex(globalIndex)}
                            className={`w-full flex items-center gap-3 px-4 py-3 mx-2 rounded-lg
                              transition-colors text-left max-w-[calc(100%-16px)]
                              ${
                                isSelected
                                  ? "bg-blue-500/20 text-white"
                                  : "text-gray-300 hover:bg-gray-800/50"
                              }`}
                          >
                            <command.icon
                              className={`w-5 h-5 ${
                                isSelected ? "text-blue-400" : "text-gray-500"
                              }`}
                            />
                            <div className="flex-1 min-w-0">
                              <div className="font-medium">{command.title}</div>
                              {command.subtitle && (
                                <div className="text-sm text-gray-500 truncate">
                                  {command.subtitle}
                                </div>
                              )}
                            </div>
                            {command.shortcut && (
                              <kbd
                                className={`px-2 py-1 text-xs rounded ${
                                  isSelected
                                    ? "bg-blue-500/30 text-blue-300"
                                    : "bg-gray-700 text-gray-400"
                                }`}
                              >
                                {command.shortcut}
                              </kbd>
                            )}
                            {isSelected && (
                              <ArrowRight className="w-4 h-4 text-blue-400" />
                            )}
                          </button>
                        );
                      })}
                    </div>
                  ))
                )}
              </div>

              {/* Footer */}
              <div className="flex items-center justify-between px-4 py-3 
                bg-gray-800/30 border-t border-gray-800/50 text-xs text-gray-500">
                <div className="flex items-center gap-4">
                  <span className="flex items-center gap-1">
                    <kbd className="px-1.5 py-0.5 bg-gray-700 rounded">↑↓</kbd>
                    <span>to navigate</span>
                  </span>
                  <span className="flex items-center gap-1">
                    <kbd className="px-1.5 py-0.5 bg-gray-700 rounded">↵</kbd>
                    <span>to select</span>
                  </span>
                </div>
                <span>{filteredCommands.length} commands</span>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}

// Quick search bar for navbar
export function QuickSearchButton() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <>
      <button
        onClick={() => setIsOpen(true)}
        className="w-10 h-10 rounded-xl bg-gray-800/50 hover:bg-gray-700/50 
          border border-gray-700/50 flex items-center justify-center
          text-gray-400 hover:text-white transition-colors"
      >
        <Search className="w-4 h-4" />
      </button>
      <CommandPalette />
    </>
  );
}
