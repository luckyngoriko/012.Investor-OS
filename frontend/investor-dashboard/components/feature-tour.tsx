"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  X, 
  ChevronRight, 
  ChevronLeft, 
  Sparkles,
  TrendingUp,
  Target,
  Shield,
  PieChart,
  RefreshCw,
  CheckCircle2,
  Play,
} from "lucide-react";

const tourSteps = [
  {
    id: "welcome",
    title: "Welcome to Investor OS",
    description: "Your AI-powered autonomous trading platform. Here's what you can do right now.",
    icon: Sparkles,
    highlight: null,
    color: "blue",
  },
  {
    id: "ai-proposals",
    title: "AI Trade Proposals",
    description: "Our AI generates trade proposals with CQ (Conviction Quotient) scores. Review them and click Confirm to execute or Reject to dismiss.",
    icon: Target,
    highlight: "ai-proposals-section",
    color: "emerald",
  },
  {
    id: "portfolio",
    title: "Portfolio Overview",
    description: "Track your P&L, win rate, and Sharpe ratio in real-time. View sector allocation and position weights.",
    icon: PieChart,
    highlight: "portfolio-chart",
    color: "purple",
  },
  {
    id: "positions",
    title: "Position Management",
    description: "Monitor all your positions with real-time price updates. Click on a position to see details or close it.",
    icon: TrendingUp,
    highlight: "positions-table",
    color: "blue",
  },
  {
    id: "risk",
    title: "Risk Dashboard",
    description: "Check your risk metrics: VaR, concentration alerts, and kill switch status. All updated in real-time.",
    icon: Shield,
    highlight: "risk-panel",
    color: "amber",
  },
  {
    id: "actions",
    title: "Quick Actions",
    description: "Access common tasks: New Trade, Run Backtest, Export Report. Keyboard shortcuts available (⌘+key).",
    icon: RefreshCw,
    highlight: "quick-actions",
    color: "cyan",
  },
];

interface FeatureTourProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete?: () => void;
}

export function FeatureTour({ isOpen, onClose, onComplete }: FeatureTourProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [hasSeenTour, setHasSeenTour] = useState(false);

  useEffect(() => {
    // Check if user has seen tour before
    const seen = localStorage.getItem("investor-os-tour-seen");
    if (seen) {
      setHasSeenTour(true);
    }
  }, []);

  const handleNext = () => {
    if (currentStep < tourSteps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      handleComplete();
    }
  };

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleComplete = () => {
    localStorage.setItem("investor-os-tour-seen", "true");
    setHasSeenTour(true);
    onComplete?.();
    onClose();
  };

  const handleSkip = () => {
    localStorage.setItem("investor-os-tour-seen", "true");
    onClose();
  };

  const step = tourSteps[currentStep];
  const Icon = step.icon;
  const progress = ((currentStep + 1) / tourSteps.length) * 100;

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        {/* Backdrop */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="absolute inset-0 bg-black/60 backdrop-blur-sm"
          onClick={handleSkip}
        />

        {/* Tour Card */}
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 20 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: 20 }}
          className="relative w-full max-w-lg glass-card rounded-2xl p-6 overflow-hidden"
        >
          {/* Progress Bar */}
          <div className="absolute top-0 left-0 right-0 h-1 bg-gray-800">
            <motion.div
              className="h-full bg-gradient-to-r from-blue-500 to-cyan-400"
              initial={{ width: 0 }}
              animate={{ width: `${progress}%` }}
              transition={{ duration: 0.3 }}
            />
          </div>

          {/* Close Button */}
          <button
            onClick={handleSkip}
            className="absolute top-4 right-4 p-2 text-gray-500 hover:text-white 
              hover:bg-gray-800 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>

          {/* Content */}
          <div className="pt-4">
            {/* Icon */}
            <motion.div
              key={step.id}
              initial={{ scale: 0.8, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              transition={{ type: "spring", stiffness: 300 }}
              className={`w-16 h-16 rounded-2xl flex items-center justify-center mb-6
                ${step.color === "blue" ? "bg-blue-500/20 text-blue-400" : ""}
                ${step.color === "emerald" ? "bg-emerald-500/20 text-emerald-400" : ""}
                ${step.color === "purple" ? "bg-purple-500/20 text-purple-400" : ""}
                ${step.color === "amber" ? "bg-amber-500/20 text-amber-400" : ""}
                ${step.color === "cyan" ? "bg-cyan-500/20 text-cyan-400" : ""}
              `}
            >
              <Icon className="w-8 h-8" />
            </motion.div>

            {/* Title & Description */}
            <motion.div
              key={`${step.id}-text`}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
            >
              <h3 className="text-2xl font-bold text-white mb-3">{step.title}</h3>
              <p className="text-gray-400 leading-relaxed">{step.description}</p>
            </motion.div>

            {/* Feature Preview (for some steps) */}
            {step.id === "ai-proposals" && (
              <motion.div
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ delay: 0.2 }}
                className="mt-6 p-4 rounded-xl bg-gray-800/50 border border-gray-700"
              >
                <div className="flex items-center gap-3 mb-3">
                  <div className="w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                    <TrendingUp className="w-4 h-4 text-emerald-400" />
                  </div>
                  <div>
                    <p className="font-medium text-white">AAPL Buy Proposal</p>
                    <p className="text-xs text-gray-500">CQ Score: 87%</p>
                  </div>
                </div>
                <div className="flex gap-2">
                  <button className="flex-1 py-2 bg-emerald-600 hover:bg-emerald-500 text-white 
                    text-sm font-medium rounded-lg transition-colors">
                    Confirm
                  </button>
                  <button className="flex-1 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 
                    text-sm font-medium rounded-lg transition-colors">
                    Reject
                  </button>
                </div>
              </motion.div>
            )}

            {/* Step Indicators */}
            <div className="flex justify-center gap-2 mt-8">
              {tourSteps.map((_, index) => (
                <button
                  key={index}
                  onClick={() => setCurrentStep(index)}
                  className={`w-2 h-2 rounded-full transition-all duration-300
                    ${index === currentStep ? "w-6 bg-blue-500" : "bg-gray-600 hover:bg-gray-500"}`}
                />
              ))}
            </div>

            {/* Navigation */}
            <div className="flex items-center justify-between mt-8">
              <button
                onClick={handlePrev}
                disabled={currentStep === 0}
                className="flex items-center gap-2 px-4 py-2 text-gray-400 hover:text-white 
                  disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              >
                <ChevronLeft className="w-5 h-5" />
                Back
              </button>

              <div className="flex gap-3">
                {currentStep === tourSteps.length - 1 ? (
                  <button
                    onClick={handleComplete}
                    className="flex items-center gap-2 px-6 py-2.5 bg-gradient-to-r from-emerald-500 
                      to-emerald-600 hover:from-emerald-400 hover:to-emerald-500 text-white 
                      font-medium rounded-lg transition-all"
                  >
                    <CheckCircle2 className="w-5 h-5" />
                    Get Started
                  </button>
                ) : (
                  <button
                    onClick={handleNext}
                    className="flex items-center gap-2 px-6 py-2.5 bg-blue-600 hover:bg-blue-500 
                      text-white font-medium rounded-lg transition-colors"
                  >
                    Next
                    <ChevronRight className="w-5 h-5" />
                  </button>
                )}
              </div>
            </div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
}

// ============================================
// TOUR TRIGGER BUTTON
// ============================================

export function TourTriggerButton() {
  const [showTour, setShowTour] = useState(false);

  return (
    <>
      <motion.button
        onClick={() => setShowTour(true)}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        className="flex items-center gap-2 px-4 py-2 bg-gray-800/50 hover:bg-gray-700/50 
          text-gray-300 hover:text-white text-sm font-medium rounded-lg 
          border border-gray-700/50 transition-all"
      >
        <Play className="w-4 h-4" />
        Take Tour
      </motion.button>

      <FeatureTour isOpen={showTour} onClose={() => setShowTour(false)} />
    </>
  );
}

// ============================================
// FIRST-TIME WELCOME MODAL
// ============================================

export function FirstTimeWelcome() {
  const [isOpen, setIsOpen] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    const hasSeenWelcome = localStorage.getItem("investor-os-welcome-seen");
    if (!hasSeenWelcome) {
      // Small delay for better UX
      const timer = setTimeout(() => {
        setIsOpen(true);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, []);

  const handleStart = () => {
    localStorage.setItem("investor-os-welcome-seen", "true");
    setIsOpen(false);
  };

  // Prevent hydration mismatch - don't render until mounted on client
  if (!mounted || !isOpen) return null;

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="absolute inset-0 bg-black/80 backdrop-blur-md"
          onClick={handleStart}
        />

        <motion.div
          initial={{ opacity: 0, scale: 0.9, y: 30 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.9, y: 30 }}
          transition={{ type: "spring", stiffness: 300, damping: 25 }}
          className="relative w-full max-w-2xl glass-card rounded-3xl p-8 overflow-hidden"
        >
          {/* Background Glow */}
          <div className="absolute top-0 right-0 w-96 h-96 bg-blue-500/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2" />
          <div className="absolute bottom-0 left-0 w-96 h-96 bg-emerald-500/10 rounded-full blur-3xl translate-y-1/2 -translate-x-1/2" />

          <div className="relative">
            {/* Header */}
            <div className="text-center mb-8">
              <motion.div
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                transition={{ type: "spring", stiffness: 400, delay: 0.1 }}
                className="w-20 h-20 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-400 
                  flex items-center justify-center shadow-lg shadow-blue-500/25"
              >
                <Sparkles className="w-10 h-10 text-white" />
              </motion.div>

              <motion.h1
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 }}
                className="text-3xl font-bold text-white mb-3"
              >
                Welcome to Investor OS
              </motion.h1>

              <motion.p
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.3 }}
                className="text-gray-400 text-lg"
              >
                Your AI-powered autonomous trading platform
              </motion.p>
            </div>

            {/* Key Features Grid */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.4 }}
              className="grid grid-cols-2 gap-4 mb-8"
            >
              {[
                { icon: Target, label: "AI Trade Proposals", desc: "Smart suggestions with CQ scores" },
                { icon: PieChart, label: "Portfolio Tracking", desc: "Real-time P&L & analytics" },
                { icon: Shield, label: "Risk Management", desc: "VaR, kill switch & alerts" },
                { icon: RefreshCw, label: "Backtesting", desc: "Test strategies historically" },
              ].map((item, index) => (
                <div
                  key={item.label}
                  className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50"
                >
                  <item.icon className="w-6 h-6 text-blue-400 mb-2" />
                  <p className="font-medium text-white text-sm">{item.label}</p>
                  <p className="text-xs text-gray-500">{item.desc}</p>
                </div>
              ))}
            </motion.div>

            {/* Demo Account Notice */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.5 }}
              className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 mb-8"
            >
              <p className="text-sm text-amber-200">
                <strong className="text-amber-400">Demo Mode:</strong> You're viewing with simulated data. 
                Connect your broker to start live trading.
              </p>
            </motion.div>

            {/* CTA Button */}
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.6 }}
            >
              <button
                onClick={handleStart}
                className="w-full py-4 bg-gradient-to-r from-blue-600 to-cyan-500 hover:from-blue-500 
                  hover:to-cyan-400 text-white font-semibold rounded-xl transition-all
                  shadow-lg shadow-blue-500/25 hover:shadow-blue-500/40"
              >
                Start Exploring
              </button>
              <p className="text-center text-gray-500 text-sm mt-3">
                Press ⌘+? anytime for keyboard shortcuts
              </p>
            </motion.div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
}
