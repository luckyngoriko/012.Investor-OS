"use client";

import React from "react";
import { motion } from "framer-motion";
import {
  Package,
  Search,
  FileX,
  Inbox,
  TrendingUp,
  Target,
  Sparkles,
  Plus,
  ArrowRight,
  BookOpen,
  Lightbulb,
} from "lucide-react";
import Link from "next/link";

interface EmptyStateProps {
  icon?: React.ElementType;
  title: string;
  description: string;
  action?: {
    label: string;
    href?: string;
    onClick?: () => void;
  };
  secondaryAction?: {
    label: string;
    href?: string;
    onClick?: () => void;
  };
  size?: "sm" | "md" | "lg";
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  secondaryAction,
  size = "md",
}: EmptyStateProps) {
  const sizes = {
    sm: "py-8",
    md: "py-12",
    lg: "py-16",
  };

  const iconSizes = {
    sm: "w-12 h-12",
    md: "w-16 h-16",
    lg: "w-20 h-20",
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className={`flex flex-col items-center justify-center text-center px-4 ${sizes[size]}`}
    >
      {Icon && (
        <div
          className={`${iconSizes[size]} rounded-2xl bg-gray-800/50 
          flex items-center justify-center mb-4`}
        >
          <Icon className="w-1/2 h-1/2 text-gray-500" />
        </div>
      )}
      <h3
        className={`font-semibold text-white mb-2 ${
          size === "sm" ? "text-base" : size === "md" ? "text-lg" : "text-xl"
        }`}
      >
        {title}
      </h3>
      <p className="text-gray-400 max-w-sm mb-6">{description}</p>

      <div className="flex flex-wrap items-center justify-center gap-3">
        {action &&
          (action.href ? (
            <Link
              href={action.href}
              className="inline-flex items-center gap-2 px-4 py-2 
                bg-blue-600 hover:bg-blue-500 text-white rounded-lg
                transition-colors font-medium"
            >
              <Plus className="w-4 h-4" />
              {action.label}
            </Link>
          ) : (
            <button
              onClick={action.onClick}
              className="inline-flex items-center gap-2 px-4 py-2 
                bg-blue-600 hover:bg-blue-500 text-white rounded-lg
                transition-colors font-medium"
            >
              <Plus className="w-4 h-4" />
              {action.label}
            </button>
          ))}

        {secondaryAction &&
          (secondaryAction.href ? (
            <Link
              href={secondaryAction.href}
              className="inline-flex items-center gap-2 px-4 py-2 
                text-gray-400 hover:text-white transition-colors"
            >
              {secondaryAction.label}
              <ArrowRight className="w-4 h-4" />
            </Link>
          ) : (
            <button
              onClick={secondaryAction.onClick}
              className="inline-flex items-center gap-2 px-4 py-2 
                text-gray-400 hover:text-white transition-colors"
            >
              {secondaryAction.label}
              <ArrowRight className="w-4 h-4" />
            </button>
          ))}
      </div>
    </motion.div>
  );
}

// Predefined empty states
export function NoPositionsEmptyState() {
  return (
    <EmptyState
      icon={Package}
      title="No Active Positions"
      description="You don't have any open positions yet. Start trading to build your portfolio."
      action={{
        label: "View AI Proposals",
        href: "/proposals",
      }}
      secondaryAction={{
        label: "Learn More",
        href: "/help/trading",
      }}
      size="lg"
    />
  );
}

export function NoProposalsEmptyState() {
  return (
    <EmptyState
      icon={Target}
      title="No AI Proposals"
      description="Our AI is analyzing the market for opportunities. Check back soon for new trading ideas."
      action={{
        label: "Adjust AI Settings",
        href: "/settings/ai",
      }}
      secondaryAction={{
        label: "View Market Analysis",
        href: "/chart",
      }}
      size="lg"
    />
  );
}

export function NoSearchResultsEmptyState({ query }: { query: string }) {
  return (
    <EmptyState
      icon={Search}
      title="No Results Found"
      description={`We couldn't find anything matching "${query}". Try different keywords or check your spelling.`}
      action={{
        label: "Clear Search",
        onClick: () => window.location.reload(),
      }}
      size="md"
    />
  );
}

export function NoDataEmptyState({
  title = "No Data Available",
  description = "There's no data to display at the moment.",
}: {
  title?: string;
  description?: string;
}) {
  return (
    <EmptyState
      icon={FileX}
      title={title}
      description={description}
      size="md"
    />
  );
}

export function EmptyInboxState() {
  return (
    <EmptyState
      icon={Inbox}
      title="All Caught Up!"
      description="You have no new notifications. We'll alert you when something important happens."
      size="md"
    />
  );
}

export function FirstTimeDashboardState() {
  return (
    <div className="glass-card rounded-2xl p-8">
      <div className="flex flex-col items-center text-center">
        <div
          className="w-20 h-20 rounded-2xl bg-gradient-to-br from-blue-500/20 to-cyan-500/10 
          flex items-center justify-center mb-6"
        >
          <Sparkles className="w-10 h-10 text-blue-400" />
        </div>
        <h2 className="text-2xl font-bold text-white mb-3">
          Welcome to Investor OS!
        </h2>
        <p className="text-gray-400 max-w-md mb-8">
          Your AI-powered trading platform is ready. Let&apos;s get you started
          with a quick tour of the key features.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 w-full max-w-2xl">
          <QuickStartCard
            icon={TrendingUp}
            title="View Dashboard"
            description="Check your portfolio overview"
            href="/"
          />
          <QuickStartCard
            icon={Target}
            title="AI Proposals"
            description="Review AI trading ideas"
            href="/proposals"
          />
          <QuickStartCard
            icon={BookOpen}
            title="Read Docs"
            description="Learn how to use the platform"
            href="/docs"
          />
        </div>
      </div>
    </div>
  );
}

function QuickStartCard({
  icon: Icon,
  title,
  description,
  href,
}: {
  icon: React.ElementType;
  title: string;
  description: string;
  href: string;
}) {
  return (
    <Link
      href={href}
      className="group p-4 rounded-xl bg-gray-800/30 hover:bg-gray-800/50 
        border border-gray-700/30 hover:border-gray-600/50
        transition-all text-left"
    >
      <Icon className="w-8 h-8 text-blue-400 mb-3 group-hover:scale-110 transition-transform" />
      <h3 className="font-medium text-white mb-1">{title}</h3>
      <p className="text-sm text-gray-500">{description}</p>
    </Link>
  );
}

export function ErrorState({
  error,
  reset,
}: {
  error: Error;
  reset?: () => void;
}) {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-4 text-center">
      <div className="w-20 h-20 rounded-2xl bg-rose-500/10 flex items-center justify-center mb-6">
        <span className="text-4xl">😕</span>
      </div>
      <h2 className="text-xl font-semibold text-white mb-2">
        Something went wrong
      </h2>
      <p className="text-gray-400 max-w-md mb-2">
        {error.message || "An unexpected error occurred. Please try again."}
      </p>
      <div className="flex items-center gap-3">
        {reset && (
          <button
            onClick={reset}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white 
              rounded-lg transition-colors font-medium"
          >
            Try Again
          </button>
        )}
        <button
          onClick={() => window.location.reload()}
          className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
        >
          Reload Page
        </button>
      </div>
    </div>
  );
}

export function ComingSoonState({ feature }: { feature: string }) {
  const [requested, setRequested] = React.useState(false);

  const handleRequest = async () => {
    try {
      await fetch("/api/waitlist", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ feature }),
      });
    } catch {
      // Best-effort: waitlist endpoint may not exist yet
    }
    setRequested(true);
  };

  return (
    <EmptyState
      icon={Lightbulb}
      title={`${feature} Coming Soon`}
      description="This feature is currently in development. Stay tuned for updates!"
      action={{
        label: requested ? "Request Sent" : "Request Early Access",
        onClick: requested ? undefined : handleRequest,
      }}
      size="lg"
    />
  );
}
