import type { Metadata } from "next";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider";
import { ErrorBoundary } from "@/components/error-boundary";
import { I18nProvider } from "@/components/i18n-provider";
import { AuthProvider } from "@/lib/auth-context";
import { AppShell } from "@/components/app-shell";
import { ToastContainer } from "@/components/notification-center";

export const metadata: Metadata = {
  title: "Investor OS - AI-Powered Trading Platform",
  description:
    "Professional autonomous trading system with AI-driven decision making",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="bg"
      className="dark"
      data-scroll-behavior="smooth"
      suppressHydrationWarning
    >
      <head>
        <link rel="manifest" href="/manifest.json" />
        <meta name="theme-color" content="#0a0f1c" />
      </head>
      <body className="antialiased">
        <ErrorBoundary>
          <ThemeProvider>
            <I18nProvider>
              <AuthProvider>
                <AppShell>{children}</AppShell>

                {/* Global Components */}
                <ToastContainer />
              </AuthProvider>
            </I18nProvider>
          </ThemeProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
