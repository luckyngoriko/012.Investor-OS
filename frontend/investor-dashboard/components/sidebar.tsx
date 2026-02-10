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
} from "lucide-react";
import { useAuth, RequireRole } from "@/lib/auth-context";

// Navigation items - filtered by role
const getNavItems = (isAdmin: boolean) => [
  { 
    href: "/", 
    label: "Dashboard", 
    icon: LayoutDashboard,
    description: "Portfolio & chart",
    requiredRole: ["admin", "trader", "viewer"] as const,
  },
  { 
    href: "/chart", 
    label: "Trading Chart", 
    icon: CandlestickChart,
    description: "Advanced chart",
    requiredRole: ["admin", "trader"] as const,
  },
  { 
    href: "/positions", 
    label: "Positions", 
    icon: TrendingUp,
    description: "Manage positions",
    requiredRole: ["admin", "trader"] as const,
  },
  { 
    href: "/portfolio", 
    label: "Portfolio", 
    icon: PieChart,
    description: "Analytics",
    requiredRole: ["admin", "trader", "viewer"] as const,
  },
  { 
    href: "/proposals", 
    label: "AI Proposals", 
    icon: Target,
    badge: 3,
    description: "Trade ideas",
    requiredRole: ["admin", "trader"] as const,
  },
  { 
    href: "/risk", 
    label: "Risk Management", 
    icon: Shield,
    description: "VaR & alerts",
    requiredRole: ["admin", "trader"] as const,
  },
  { 
    href: "/backtest", 
    label: "Backtesting", 
    icon: RefreshCw,
    description: "Test strategies",
    requiredRole: ["admin", "trader"] as const,
  },
  { 
    href: "/journal", 
    label: "Trading Journal", 
    icon: FileText,
    description: "Log & reflect",
    disabled: true,
    requiredRole: ["admin", "trader"] as const,
  },
  // Admin section - only for admins
  ...(isAdmin ? [
    { 
      href: "/admin", 
      label: "Administration", 
      icon: Settings,
      description: "System config",
      requiredRole: ["admin"] as const,
      isAdmin: true,
    },
  ] : []),
];

export default function Sidebar() {
  const pathname = usePathname();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [hoveredItem, setHoveredItem] = useState<string | null>(null);
  const { user, logout, hasRole } = useAuth();
  
  const isAdmin = hasRole("admin");
  const navItems = getNavItems(isAdmin);

  const handleLogout = () => {
    logout();
    window.location.href = "/login";
  };

  return (
    <>
      {/* Mobile Menu Button */}
      <button
        onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
        className="lg:hidden fixed top-4 left-4 z-50 p-3 rounded-xl bg-gray-900/90 border border-gray-800 text-white"
      >
        {isMobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
      </button>

      {/* Mobile Overlay */}
      <AnimatePresence>
        {isMobileMenuOpen && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="lg:hidden fixed inset-0 z-40 bg-black/80 backdrop-blur-sm"
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
          fixed lg:fixed left-0 top-0 h-full z-40
          w-72 bg-gradient-to-b from-[#0a0f1c] via-[#111827] to-[#0a0f1c]
          border-r border-gray-800/50
          transform transition-transform duration-300 ease-in-out
          lg:translate-x-0
          ${isMobileMenuOpen ? "translate-x-0" : "-translate-x-full"}
        `}
      >
        <div className="flex flex-col h-full">
          {/* Logo */}
          <div className="p-6 border-b border-gray-800/50">
            <Link href="/" className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-600 to-cyan-500 flex items-center justify-center shadow-lg shadow-blue-500/20">
                <Sparkles className="w-5 h-5 text-white" />
              </div>
              <div>
                <h1 className="text-lg font-bold bg-gradient-to-r from-white to-gray-300 bg-clip-text text-transparent">
                  Investor OS
                </h1>
                <p className="text-xs text-gray-500">AI-Powered Trading</p>
              </div>
            </Link>
          </div>

          {/* User Role Badge */}
          {user && (
            <div className="px-4 py-2">
              <div className={`px-3 py-1.5 rounded-lg text-xs font-medium text-center
                ${user.role === "admin" ? "bg-purple-500/20 text-purple-400 border border-purple-500/30" : ""}
                ${user.role === "trader" ? "bg-blue-500/20 text-blue-400 border border-blue-500/30" : ""}
                ${user.role === "viewer" ? "bg-gray-500/20 text-gray-400 border border-gray-500/30" : ""}
              `}>
                {user.role === "admin" && "Administrator"}
                {user.role === "trader" && "Trader"}
                {user.role === "viewer" && "Viewer"}
              </div>
            </div>
          )}

          {/* Navigation */}
          <nav className="flex-1 px-4 py-4 space-y-1 overflow-y-auto">
            {navItems.map((item) => {
              // Check if user has required role
              const currentRole = user?.role || "viewer";
              if (!item.requiredRole.includes(currentRole as any)) {
                return null;
              }

              const isActive = pathname === item.href;
              const Icon = item.icon;
              const isHovered = hoveredItem === item.href;
              const isAdminItem = item.isAdmin;

              return (
                <div
                  key={item.href}
                  onMouseEnter={() => setHoveredItem(item.href)}
                  onMouseLeave={() => setHoveredItem(null)}
                >
                  <Link
                    href={item.disabled ? "#" : item.href}
                    onClick={() => item.disabled ? alert("Coming soon!") : setIsMobileMenuOpen(false)}
                    className={`flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200 group
                      ${isActive 
                        ? `bg-blue-600/20 text-blue-400 border border-blue-500/30` 
                        : "text-gray-400 hover:text-white hover:bg-gray-800/50"
                      }
                      ${item.disabled ? "opacity-50 cursor-not-allowed" : ""}
                      ${isAdminItem ? "border-l-2 border-l-purple-500" : ""}
                    `}
                  >
                    <div className={`
                      w-10 h-10 rounded-lg flex items-center justify-center transition-colors
                      ${isActive ? "bg-blue-500/20" : "bg-gray-800/50 group-hover:bg-gray-700/50"}
                      ${isAdminItem ? "bg-purple-500/10" : ""}
                    `}>
                      <Icon className={`w-5 h-5 ${isAdminItem ? "text-purple-400" : ""}`} />
                    </div>
                    
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{item.label}</span>
                        {item.badge && (
                          <span className="px-2 py-0.5 text-xs font-bold bg-red-500 text-white rounded-full">
                            {item.badge}
                          </span>
                        )}
                        {item.disabled && (
                          <span className="text-xs text-gray-500">(Soon)</span>
                        )}
                        {isAdminItem && (
                          <span className="px-1.5 py-0.5 text-[10px] font-bold bg-purple-500/20 text-purple-400 rounded">
                            ADMIN
                          </span>
                        )}
                      </div>
                      
                      {/* Description on hover */}
                      <AnimatePresence>
                        {isHovered && (
                          <motion.p
                            initial={{ opacity: 0, height: 0 }}
                            animate={{ opacity: 1, height: "auto" }}
                            exit={{ opacity: 0, height: 0 }}
                            className="text-xs text-gray-500 mt-0.5"
                          >
                            {item.description}
                          </motion.p>
                        )}
                      </AnimatePresence>
                    </div>

                    {isActive && (
                      <motion.div
                        layoutId="activeIndicator"
                        className="w-1.5 h-1.5 rounded-full bg-blue-400"
                      />
                    )}
                    
                    {!isActive && !item.disabled && (
                      <ChevronRight className={`
                        w-4 h-4 transition-all
                        ${isHovered ? "opacity-100 translate-x-0" : "opacity-0 -translate-x-2"}
                      `} />
                    )}
                  </Link>
                </div>
              );
            })}
          </nav>

          {/* Bottom Section */}
          <div className="p-4 border-t border-gray-800/50 space-y-3">
            {/* User Info */}
            {user && (
              <div className="flex items-center gap-3 px-4 py-3 rounded-xl bg-gray-800/30">
                <div className={`w-10 h-10 rounded-full flex items-center justify-center font-semibold text-sm
                  ${user.role === "admin" ? "bg-purple-500 text-white" : ""}
                  ${user.role === "trader" ? "bg-blue-500 text-white" : ""}
                  ${user.role === "viewer" ? "bg-gray-500 text-white" : ""}
                `}>
                  {user.avatar}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-white text-sm truncate">{user.name}</p>
                  <p className="text-xs text-gray-500">{user.email}</p>
                </div>
              </div>
            )}

            {/* Logout */}
            <button 
              onClick={handleLogout}
              className="flex items-center gap-3 w-full px-4 py-3 rounded-xl text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 transition-all"
            >
              <LogOut className="w-5 h-5" />
              <span className="font-medium">Sign Out</span>
            </button>
          </div>
        </div>
      </motion.aside>
    </>
  );
}
