"use client";

import type { ReactNode } from "react";
import { usePathname } from "next/navigation";
import ImprovedSidebar from "@/components/sidebar-improved";
import Breadcrumbs from "@/components/breadcrumbs";
import { CommandPalette } from "@/components/command-palette";

const SHELLLESS_ROUTES = ["/login"];

type AppShellProps = {
  children: ReactNode;
};

export function AppShell({ children }: AppShellProps) {
  const pathname = usePathname();
  const hideShell =
    pathname !== null &&
    SHELLLESS_ROUTES.some(
      (r) => pathname === r || pathname.startsWith(r + "/"),
    );

  if (hideShell) {
    return <main className="min-h-screen">{children}</main>;
  }

  return (
    <>
      <div className="flex min-h-screen bg-[#0a0f1c]">
        <ImprovedSidebar />

        <div className="flex min-w-0 flex-1 flex-col">
          <Breadcrumbs />

          <main className="flex-1 overflow-auto">{children}</main>
        </div>
      </div>

      <CommandPalette />
    </>
  );
}
