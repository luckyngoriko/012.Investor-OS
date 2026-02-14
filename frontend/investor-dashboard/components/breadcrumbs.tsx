"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { ChevronRight, Home } from "lucide-react";

// Page title mapping
const pageTitles: Record<string, string> = {
  "/": "Dashboard",
  "/chart": "Trading Chart",
  "/positions": "Positions",
  "/portfolio": "Portfolio",
  "/proposals": "AI Proposals",
  "/risk": "Risk Management",
  "/portfolio-opt": "Portfolio Optimization",
  "/strategy": "Strategy Selector",
  "/tax": "Tax & Compliance",
  "/monitoring": "Monitoring",
  "/security": "Security",
  "/ai-train": "AI Training",
  "/deployment": "Deployment",
  "/backtest": "Backtesting",
  "/journal": "Trading Journal",
  "/settings": "Settings",
  "/admin": "Administration",
  "/login": "Login",
};

export default function Breadcrumbs() {
  const pathname = usePathname();
  
  // Don't show breadcrumbs on login page
  if (pathname === "/login") return null;
  
  // Build breadcrumb items
  const items = [
    { href: "/", label: "Home", icon: Home },
  ];
  
  // Add current page
  if (pathname !== "/") {
    const title = pageTitles[pathname] || pathname.split("/").pop()?.replace(/-/g, " ") || "Page";
    items.push({ 
      href: pathname, 
      label: title.charAt(0).toUpperCase() + title.slice(1),
      icon: ChevronRight,
    });
  }

  return (
    <nav className="flex items-center gap-2 px-4 py-3 text-sm text-gray-400 border-b border-gray-800/50 bg-[#0a0f1c]/50">
      {items.map((item, index) => {
        const isLast = index === items.length - 1;
        const Icon = item.icon;
        
        return (
          <div key={item.href} className="flex items-center gap-2">
            {index > 0 && (
              <ChevronRight className="w-4 h-4 text-gray-600" />
            )}
            
            {isLast ? (
              <span className="font-medium text-white">
                {Icon && <Icon className="w-4 h-4 inline mr-1" />}
                {item.label}
              </span>
            ) : (
              <Link
                href={item.href}
                className="hover:text-blue-400 transition-colors flex items-center gap-1"
              >
                {Icon && <Icon className="w-4 h-4" />}
                {item.label}
              </Link>
            )}
          </div>
        );
      })}
    </nav>
  );
}
