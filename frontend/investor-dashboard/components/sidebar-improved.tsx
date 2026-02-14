"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import {
  LayoutDashboard,
  TrendingUp,
  PieChart,
  Target,
  Shield,
  RefreshCw,
  FileText,
  Settings,
  LogOut,
  Menu,
  X,
  ChevronRight,
  Sparkles,
  CandlestickChart,
  Lock,
  Brain,
  Calculator,
  Activity,
  Server,
  BarChart3,
  ChevronDown,
  Globe,
} from "lucide-react";
import { useAuth } from "@/lib/auth-context";
import { useTranslations } from "@/components/i18n-provider";
import { useUserLocale } from "@/i18n/client";
import { LucideIcon } from "lucide-react";
import { CompactLanguageSelector } from "./language-selector";
import { NotificationCenter } from "./notification-center";
import { ThemeToggle } from "./theme-provider";
import { CommandPalette } from "./command-palette";

// Navigation item type
type NavItem = {
  href: string;
  icon: LucideIcon;
  badge?: number;
  disabled?: boolean;
  translationKey: string;
  descTranslationKey: string;
};

type NavGroup = {
  titleKey: string;
  items: NavItem[];
};

// Navigation groups with translation keys
const navGroupsConfig: NavGroup[] = [
  {
    titleKey: "navGroups.main",
    items: [
      { 
        href: "/", 
        icon: LayoutDashboard,
        translationKey: "dashboard",
        descTranslationKey: "dashboard",
      },
      { 
        href: "/portfolio", 
        icon: PieChart,
        translationKey: "portfolio",
        descTranslationKey: "portfolio",
      },
      { 
        href: "/positions", 
        icon: TrendingUp,
        translationKey: "positions",
        descTranslationKey: "positions",
      },
      { 
        href: "/chart", 
        icon: CandlestickChart,
        translationKey: "chart",
        descTranslationKey: "chart",
      },
    ]
  },
  {
    titleKey: "navGroups.aiTrading",
    items: [
      { 
        href: "/proposals", 
        icon: Target,
        badge: 3,
        translationKey: "proposals",
        descTranslationKey: "proposals",
      },
      { 
        href: "/strategy", 
        icon: Brain,
        translationKey: "strategy",
        descTranslationKey: "strategy",
      },
      { 
        href: "/backtest", 
        icon: RefreshCw,
        translationKey: "backtest",
        descTranslationKey: "backtest",
      },
      { 
        href: "/ai-train", 
        icon: Sparkles,
        translationKey: "aiTraining",
        descTranslationKey: "aiTraining",
      },
    ]
  },
  {
    titleKey: "navGroups.management",
    items: [
      { 
        href: "/risk", 
        icon: Shield,
        translationKey: "risk",
        descTranslationKey: "risk",
      },
      { 
        href: "/portfolio-opt", 
        icon: BarChart3,
        translationKey: "optimization",
        descTranslationKey: "optimization",
      },
      { 
        href: "/tax", 
        icon: Calculator,
        translationKey: "tax",
        descTranslationKey: "tax",
      },
      { 
        href: "/journal", 
        icon: FileText,
        translationKey: "journal",
        descTranslationKey: "journal",
        disabled: true,
      },
    ]
  },
  {
    titleKey: "navGroups.system",
    items: [
      { 
        href: "/monitoring", 
        icon: Activity,
        translationKey: "monitoring",
        descTranslationKey: "monitoring",
      },
      { 
        href: "/security", 
        icon: Lock,
        translationKey: "security",
        descTranslationKey: "security",
      },
      { 
        href: "/deployment", 
        icon: Server,
        translationKey: "deployment",
        descTranslationKey: "deployment",
      },
      { 
        href: "/settings", 
        icon: Settings,
        translationKey: "settings",
        descTranslationKey: "settings",
      },
    ]
  },
];

export default function ImprovedSidebar() {
  const pathname = usePathname();
  const { user, logout } = useAuth();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<string[]>(["navGroups.main"]);
  
  const { t: tNav } = useTranslations("navigation");
  const { t: tNavDesc } = useTranslations("navDescriptions");
  const { t: tGroups } = useTranslations();
  const { t: tAuth } = useTranslations("auth");
  const [locale] = useUserLocale();

  const toggleGroup = (titleKey: string) => {
    setExpandedGroups(prev => 
      prev.includes(titleKey) 
        ? prev.filter(t => t !== titleKey)
        : [...prev, titleKey]
    );
  };

  const isActive = (href: string) => {
    if (href === "/") {
      return pathname === "/";
    }
    return pathname?.startsWith(href);
  };

  return (
    <>
      {/* Mobile Header */}
      <div className="lg:hidden fixed top-0 left-0 right-0 z-50 px-4 py-3 bg-[#0a0f1c]/95 backdrop-blur-lg border-b border-gray-800/50">
        <div className="flex items-center justify-between">
          <button
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            className="p-2 rounded-xl bg-gray-800/50 border border-gray-700 text-white"
          >
            {isMobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
          </button>
          
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-600 to-cyan-500 flex items-center justify-center">
              <Sparkles className="w-4 h-4 text-white" />
            </div>
            <span className="font-bold text-white">Investor OS</span>
          </div>

          <div className="flex items-center gap-2">
            <CommandPalette />
            <NotificationCenter />
            <CompactLanguageSelector />
          </div>
        </div>
      </div>

      {/* Mobile Overlay */}
      <AnimatePresence>
        {isMobileMenuOpen && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="lg:hidden fixed inset-0 z-30 bg-black/80 backdrop-blur-sm"
            onClick={() => setIsMobileMenuOpen(false)}
          />
        )}
      </AnimatePresence>

      {/* Sidebar */}
      <motion.aside
        initial={false}
        animate={{ 
          x: isMobileMenuOpen ? 0 : undefined 
        }}
        className={`
          fixed lg:sticky left-0 top-0 h-screen z-40
          w-72 bg-gradient-to-b from-[#0a0f1c] via-[#111827] to-[#0a0f1c]
          border-r border-gray-800/50
          transform transition-transform duration-300 ease-in-out
          lg:translate-x-0 flex-shrink-0
          ${isMobileMenuOpen ? "translate-x-0" : "-translate-x-full"}
          pt-14 lg:pt-0
        `}
      >
        <div className="flex flex-col h-full overflow-hidden">
          {/* Logo - Desktop only */}
          <div className="hidden lg:flex p-6 border-b border-gray-800/50">
            <Link href="/" className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-600 to-cyan-500 flex items-center justify-center shadow-lg shadow-blue-500/20">
                <Sparkles className="w-5 h-5 text-white" />
              </div>
              <div>
                <span className="text-xl font-bold bg-gradient-to-r from-white to-gray-400 bg-clip-text text-transparent">
                  Investor OS
                </span>
                <p className="text-xs text-gray-500">v3.0 Professional</p>
              </div>
            </Link>
          </div>

          {/* Navigation Groups */}
          <nav className="flex-1 overflow-y-auto py-4 px-3 space-y-2 scrollbar-thin">
            {navGroupsConfig.map((group) => (
              <div key={group.titleKey} className="mb-4">
                {/* Group Header */}
                <button
                  onClick={() => toggleGroup(group.titleKey)}
                  className="w-full flex items-center justify-between px-3 py-2 
                    text-xs font-semibold text-gray-500 uppercase tracking-wider
                    hover:text-gray-400 transition-colors"
                >
                  <span>{tGroups(group.titleKey)}</span>
                  <ChevronDown 
                    className={`w-3 h-3 transition-transform duration-200
                      ${expandedGroups.includes(group.titleKey) ? "rotate-180" : ""}`} 
                  />
                </button>

                {/* Group Items */}
                <AnimatePresence>
                  {expandedGroups.includes(group.titleKey) && (
                    <motion.div
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: "auto", opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      transition={{ duration: 0.2 }}
                      className="overflow-hidden"
                    >
                      <div className="space-y-1 mt-1">
                        {group.items.map((item) => {
                          const active = isActive(item.href);
                          const label = tNav(item.translationKey);
                          const description = tNavDesc(item.descTranslationKey);
                          
                          return (
                            <Link
                              key={item.href}
                              href={item.disabled ? "#" : item.href}
                              className={`
                                group flex items-center gap-3 px-3 py-2.5 rounded-xl
                                transition-all duration-200 relative
                                ${active 
                                  ? "bg-gradient-to-r from-blue-600/20 to-cyan-600/10 text-white shadow-lg shadow-blue-500/10" 
                                  : "text-gray-400 hover:text-white hover:bg-gray-800/50"
                                }
                                ${item.disabled ? "opacity-50 cursor-not-allowed" : ""}
                              `}
                            >
                              {/* Active indicator */}
                              {active && (
                                <motion.div
                                  layoutId="activeNav"
                                  className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 
                                    bg-gradient-to-b from-blue-500 to-cyan-500 rounded-r-full"
                                />
                              )}

                              <item.icon className={`
                                w-5 h-5 transition-colors
                                ${active ? "text-blue-400" : "text-gray-500 group-hover:text-gray-300"}
                              `} />
                              
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                  <span className="font-medium text-sm truncate">{label}</span>
                                  {item.badge && (
                                    <span className="px-1.5 py-0.5 text-[10px] font-bold 
                                      bg-blue-500 text-white rounded-full">
                                      {item.badge}
                                    </span>
                                  )}
                                </div>
                                <p className="text-[10px] text-gray-600 group-hover:text-gray-500 
                                  truncate transition-colors">
                                  {description}
                                </p>
                              </div>

                              {active && (
                                <ChevronRight className="w-4 h-4 text-blue-400 flex-shrink-0" />
                              )}
                            </Link>
                          );
                        })}
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            ))}
          </nav>

          {/* Language Selector - Desktop */}
          <div className="hidden lg:block px-4 py-2 border-t border-gray-800/50">
            <div className="flex items-center justify-between">
              <span className="text-xs text-gray-500 flex items-center gap-1">
                <Globe className="w-3 h-3" />
                {tGroups("common.language")}
              </span>
              <CompactLanguageSelector />
            </div>
          </div>

          {/* Theme Toggle - Desktop */}
          <div className="hidden lg:block px-4 py-2 border-t border-gray-800/50">
            <div className="flex items-center justify-between">
              <span className="text-xs text-gray-500">
                {tGroups("common.theme")}
              </span>
              <ThemeToggle variant="switch" />
            </div>
          </div>

          {/* User Section */}
          {user && (
            <div className="p-4 border-t border-gray-800/50">
              <div className="flex items-center gap-3 mb-3">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-gray-700 to-gray-600 
                  flex items-center justify-center flex-shrink-0">
                  <span className="text-sm font-bold text-white">
                    {user.name?.charAt(0) || user.email?.charAt(0) || "U"}
                  </span>
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-white truncate">
                    {user.name || user.email}
                  </p>
                  <p className="text-xs text-gray-500 capitalize">{user.role}</p>
                </div>
              </div>
              
              <button
                onClick={logout}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-xl
                  text-gray-400 hover:text-white hover:bg-rose-500/10 
                  hover:border-rose-500/20 border border-transparent
                  transition-all text-sm"
              >
                <LogOut className="w-4 h-4" />
                <span>{tAuth("logout")}</span>
              </button>
            </div>
          )}
        </div>
      </motion.aside>
    </>
  );
}
