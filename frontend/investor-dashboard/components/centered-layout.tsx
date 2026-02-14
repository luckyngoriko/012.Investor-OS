"use client";

import { ReactNode } from "react";
import { motion } from "framer-motion";

interface CenteredLayoutProps {
  children: ReactNode;
  className?: string;
  maxWidth?: "sm" | "md" | "lg" | "xl" | "2xl" | "3xl" | "4xl" | "5xl" | "6xl" | "7xl" | "full";
  padding?: "none" | "sm" | "md" | "lg";
  centered?: boolean;
}

const maxWidthClasses = {
  sm: "max-w-screen-sm",
  md: "max-w-screen-md",
  lg: "max-w-screen-lg",
  xl: "max-w-screen-xl",
  "2xl": "max-w-screen-2xl",
  "3xl": "max-w-3xl",
  "4xl": "max-w-4xl",
  "5xl": "max-w-5xl",
  "6xl": "max-w-6xl",
  "7xl": "max-w-7xl",
  full: "max-w-full",
};

const paddingClasses = {
  none: "",
  sm: "px-4 py-4",
  md: "px-6 py-6 lg:px-8 lg:py-8",
  lg: "px-8 py-8 lg:px-12 lg:py-12",
};

/**
 * CenteredLayout - Ensures content is properly centered in the viewport
 * 
 * Usage:
 * <CenteredLayout maxWidth="7xl" padding="md" centered>
 *   <YourContent />
 * </CenteredLayout>
 */
export function CenteredLayout({
  children,
  className = "",
  maxWidth = "7xl",
  padding = "md",
  centered = true,
}: CenteredLayoutProps) {
  return (
    <div
      className={`
        w-full
        ${centered ? "flex justify-center" : ""}
        ${paddingClasses[padding]}
        ${className}
      `}
    >
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className={`
          w-full
          ${maxWidthClasses[maxWidth]}
          ${centered ? "mx-auto" : ""}
        `}
      >
        {children}
      </motion.div>
    </div>
  );
}

/**
 * PageContainer - Full-page wrapper with consistent spacing
 */
export function PageContainer({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <main
      className={`
        flex-1
        min-h-screen
        flex
        justify-center
        p-6
        lg:p-8
        ${className}
      `}
    >
      <div className="w-full max-w-7xl space-y-6">
        {children}
      </div>
    </main>
  );
}

/**
 * ContentSection - Consistent section spacing with animation
 */
export function ContentSection({
  children,
  className = "",
  delay = 0,
}: {
  children: ReactNode;
  className?: string;
  delay?: number;
}) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay, duration: 0.4 }}
      className={className}
    >
      {children}
    </motion.section>
  );
}

/**
 * GridLayout - Responsive grid with consistent gaps
 */
export function GridLayout({
  children,
  className = "",
  cols = { default: 1, sm: 2, lg: 3, xl: 4 },
  gap = "6",
}: {
  children: ReactNode;
  className?: string;
  cols?: {
    default?: number;
    sm?: number;
    md?: number;
    lg?: number;
    xl?: number;
  };
  gap?: "4" | "6" | "8";
}) {
  const colClasses = [
    cols.default ? `grid-cols-${cols.default}` : "",
    cols.sm ? `sm:grid-cols-${cols.sm}` : "",
    cols.md ? `md:grid-cols-${cols.md}` : "",
    cols.lg ? `lg:grid-cols-${cols.lg}` : "",
    cols.xl ? `xl:grid-cols-${cols.xl}` : "",
  ].filter(Boolean).join(" ");

  const gapClass = `gap-${gap}`;

  return (
    <div className={`grid ${colClasses} ${gapClass} ${className}`}>
      {children}
    </div>
  );
}

/**
 * FlexCenter - Flex container with centered content
 */
export function FlexCenter({
  children,
  className = "",
  direction = "row",
}: {
  children: ReactNode;
  className?: string;
  direction?: "row" | "col";
}) {
  return (
    <div
      className={`
        flex
        ${direction === "col" ? "flex-col" : "flex-row"}
        items-center
        justify-center
        ${className}
      `}
    >
      {children}
    </div>
  );
}

/**
 * ThreeColumnLayout - Left sidebar | Center content | Right panel
 */
export function ThreeColumnLayout({
  leftSidebar,
  centerContent,
  rightPanel,
  className = "",
}: {
  leftSidebar: ReactNode;
  centerContent: ReactNode;
  rightPanel?: ReactNode;
  className?: string;
}) {
  return (
    <div className={`flex min-h-screen ${className}`}>
      {/* Left Sidebar */}
      <aside className="flex-shrink-0">
        {leftSidebar}
      </aside>
      
      {/* Center Content - Always centered */}
      <main className="flex-1 flex justify-center p-6 lg:p-8 min-h-screen">
        <div className="w-full max-w-7xl">
          {centerContent}
        </div>
      </main>
      
      {/* Right Panel (optional) */}
      {rightPanel && (
        <aside className="flex-shrink-0">
          {rightPanel}
        </aside>
      )}
    </div>
  );
}
