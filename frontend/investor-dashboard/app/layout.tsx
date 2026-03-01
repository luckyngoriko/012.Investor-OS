import type { Metadata } from "next";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider";
import { ErrorBoundary } from "@/components/error-boundary";
import { I18nProvider } from "@/components/i18n-provider";
import { AuthProvider } from "@/lib/auth-context";
import ImprovedSidebar from "@/components/sidebar-improved";
import Breadcrumbs from "@/components/breadcrumbs";
import { ToastContainer } from "@/components/notification-center";
import { CommandPalette } from "@/components/command-palette";

export const metadata: Metadata = {
  title: "Investor OS - AI-Powered Trading Platform",
  description: "Professional autonomous trading system with AI-driven decision making",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="bg" className="dark" data-scroll-behavior="smooth" suppressHydrationWarning>
      <body className="antialiased">
        <ErrorBoundary>
          <ThemeProvider>
            <I18nProvider>
              <AuthProvider>
                <div className="flex min-h-screen bg-[#0a0f1c]">
                  {/* Sidebar */}
                  <ImprovedSidebar />
                  
                  {/* Main Content */}
                  <div className="flex-1 flex flex-col min-w-0">
                    {/* Breadcrumbs */}
                    <Breadcrumbs />
                    
                    {/* Page Content */}
                    <main className="flex-1 overflow-auto">
                      {children}
                    </main>
                  </div>
                </div>

                {/* Global Components */}
                <ToastContainer />
                <CommandPalette />
              </AuthProvider>
            </I18nProvider>
          </ThemeProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
