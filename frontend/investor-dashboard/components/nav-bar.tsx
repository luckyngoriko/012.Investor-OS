"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";

const navItems = [
  { href: "/", label: "Dashboard" },
  { href: "/proposals", label: "Proposals" },
  { href: "/positions", label: "Positions" },
  { href: "/journal", label: "Journal" },
  { href: "/settings", label: "Settings" },
];

export function NavBar() {
  const pathname = usePathname();

  return (
    <header className="bg-white border-b border-slate-200 sticky top-0 z-50">
      <div className="container mx-auto px-4 h-16 flex items-center justify-between">
        <div className="flex items-center gap-8">
          <Link href="/" className="flex items-center gap-2">
            <div className="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center">
              <span className="text-white font-bold text-sm">IO</span>
            </div>
            <span className="font-bold text-xl text-slate-900">Investor OS</span>
          </Link>
          
          <nav className="hidden md:flex items-center gap-1">
            {navItems.map((item) => (
              <Link key={item.href} href={item.href}>
                <Button
                  variant={pathname === item.href ? "secondary" : "ghost"}
                  size="sm"
                  className={pathname === item.href ? "bg-slate-100" : ""}
                >
                  {item.label}
                </Button>
              </Link>
            ))}
          </nav>
        </div>

        <div className="flex items-center gap-4">
          <Badge variant="outline" className="hidden sm:flex items-center gap-2">
            <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
            System Online
          </Badge>
          <Separator orientation="vertical" className="h-6 hidden sm:block" />
          <Button variant="outline" size="sm">
            Login
          </Button>
        </div>
      </div>
    </header>
  );
}
