"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { useRouter } from "next/navigation";
import {
  Eye,
  EyeOff,
  Lock,
  Mail,
  ArrowRight,
  TrendingUp,
  Shield,
  Cpu,
  Activity,
  Zap,
  AlertCircle,
} from "lucide-react";
import { useAuth } from "@/lib/auth-context";

export default function LoginPage() {
  const router = useRouter();
  const { login } = useAuth();
  const [email, setEmail] = useState("trader@investor-os.com");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [focusedInput, setFocusedInput] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);
    
    try {
      await login(email, password);
      router.push("/");
    } catch {
      setError("Invalid credentials. Please verify your email and password.");
    } finally {
      setIsLoading(false);
    }
  };

  const features = [
    { icon: Cpu, label: "AI-Powered", desc: "Smart decisions" },
    { icon: Shield, label: "Secure", desc: "Bank-grade" },
    { icon: Activity, label: "Real-time", desc: "Live data" },
    { icon: Zap, label: "Fast", desc: "Low latency" },
  ];

  return (
    <div className="min-h-screen bg-[#0a0f1c] flex">
      {/* Left Side - Form */}
      <div className="flex-1 flex items-center justify-center p-8 lg:p-12">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
          className="w-full max-w-md"
        >
          {/* Logo */}
          <div className="flex items-center gap-3 mb-8">
            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/30">
              <TrendingUp className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white">Investor OS</h1>
              <p className="text-sm text-gray-500">Professional Trading Platform</p>
            </div>
          </div>

          {/* Welcome Text */}
          <div className="mb-8">
            <h2 className="text-3xl font-bold text-white mb-2">Welcome back</h2>
            <p className="text-gray-400">Sign in to access your AI-powered trading dashboard</p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-5">
            {/* Email Input */}
            <div className="space-y-2.5">
              <label className="text-sm font-medium text-gray-300 block">Email</label>
              <motion.div
                animate={{
                  scale: focusedInput === "email" ? 1.01 : 1,
                  boxShadow:
                    focusedInput === "email"
                      ? "0 0 0 3px rgba(59, 130, 246, 0.3)"
                      : "0 0 0 0px rgba(59, 130, 246, 0)",
                }}
                className="relative"
              >
                <div className="absolute left-0 top-0 bottom-0 w-12 flex items-center justify-center pointer-events-none z-10">
                  <Mail
                    className={`w-5 h-5 transition-colors ${
                      focusedInput === "email" ? "text-blue-400" : "text-gray-500"
                    }`}
                  />
                </div>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  onFocus={() => setFocusedInput("email")}
                  onBlur={() => setFocusedInput(null)}
                  placeholder="admin@investor-os.com"
                  className="w-full pl-12 pr-4 py-4 bg-gray-800/50 border border-gray-700 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 transition-all text-base"
                  required
                />
              </motion.div>
              <p className="text-xs text-gray-500">Въведете вашия email адрес за достъп</p>
            </div>

            {/* Password Input */}
            <div className="space-y-2.5">
              <label className="text-sm font-medium text-gray-300 block">Password</label>
              <motion.div
                animate={{
                  scale: focusedInput === "password" ? 1.01 : 1,
                  boxShadow:
                    focusedInput === "password"
                      ? "0 0 0 3px rgba(59, 130, 246, 0.3)"
                      : "0 0 0 0px rgba(59, 130, 246, 0)",
                }}
                className="relative"
              >
                <div className="absolute left-0 top-0 bottom-0 w-12 flex items-center justify-center pointer-events-none z-10">
                  <Lock
                    className={`w-5 h-5 transition-colors ${
                      focusedInput === "password" ? "text-blue-400" : "text-gray-500"
                    }`}
                  />
                </div>
                <input
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  onFocus={() => setFocusedInput("password")}
                  onBlur={() => setFocusedInput(null)}
                  placeholder="Въведете паролата"
                  className="w-full pl-12 pr-12 py-4 bg-gray-800/50 border border-gray-700 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 transition-all text-base"
                  required
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-0 top-0 bottom-0 w-12 flex items-center justify-center text-gray-500 hover:text-gray-300 transition-colors z-10"
                >
                  {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                </button>
              </motion.div>
              <p className="text-xs text-gray-500">Използвайте паролата, конфигурирана за вашия акаунт.</p>
            </div>

            {/* Remember & Forgot */}
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-2 cursor-pointer group">
                <input type="checkbox" className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-500 focus:ring-blue-500/20" />
                <span className="text-sm text-gray-400 group-hover:text-gray-300 transition-colors">Remember me</span>
              </label>
              <a href="#" className="text-sm text-blue-400 hover:text-blue-300 transition-colors">
                Forgot password?
              </a>
            </div>

            {/* Submit Button */}
            <motion.button
              type="submit"
              disabled={isLoading}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              className="w-full py-4 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-500 hover:to-blue-600 text-white font-semibold rounded-xl shadow-lg shadow-blue-600/30 flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
            >
              {isLoading ? (
                <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              ) : (
                <>
                  Sign In
                  <ArrowRight className="w-5 h-5" />
                </>
              )}
            </motion.button>
          </form>

          {/* Error Message */}
          {error && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              className="mt-4 p-4 rounded-xl bg-rose-500/10 border border-rose-500/20 flex items-center gap-3"
            >
              <AlertCircle className="w-5 h-5 text-rose-400 flex-shrink-0" />
              <p className="text-sm text-rose-200">{error}</p>
            </motion.div>
          )}
        </motion.div>
      </div>

      {/* Right Side - Visual */}
      <div className="hidden lg:flex flex-1 relative overflow-hidden">
        {/* Background Gradient */}
        <div className="absolute inset-0 bg-gradient-to-br from-blue-900/20 via-[#0a0f1c] to-purple-900/20" />

        {/* Animated Grid */}
        <div className="absolute inset-0 opacity-20">
          <div
            className="absolute inset-0"
            style={{
              backgroundImage: `linear-gradient(rgba(59, 130, 246, 0.1) 1px, transparent 1px),
                linear-gradient(90deg, rgba(59, 130, 246, 0.1) 1px, transparent 1px)`,
              backgroundSize: "50px 50px",
            }}
          />
        </div>

        {/* Content */}
        <div className="relative z-10 flex flex-col justify-center px-16">
          <motion.div
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
          >
            <h2 className="text-4xl font-bold text-white mb-6">
              AI-Powered
              <br />
              <span className="text-gradient">Trading Intelligence</span>
            </h2>
            <p className="text-lg text-gray-400 mb-12 max-w-md">
              Harness the power of machine learning and real-time market analysis to make smarter investment decisions.
            </p>

            {/* Feature Grid */}
            <div className="grid grid-cols-2 gap-4">
              {features.map((feature, index) => {
                const Icon = feature.icon;
                return (
                  <motion.div
                    key={feature.label}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.5, delay: 0.4 + index * 0.1 }}
                    className="p-4 rounded-xl bg-gray-800/30 border border-gray-700/50 backdrop-blur-sm hover:bg-gray-800/50 transition-colors"
                  >
                    <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center mb-3">
                      <Icon className="w-5 h-5 text-blue-400" />
                    </div>
                    <p className="font-semibold text-white">{feature.label}</p>
                    <p className="text-sm text-gray-500">{feature.desc}</p>
                  </motion.div>
                );
              })}
            </div>
          </motion.div>

          {/* Floating Stats */}
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.8 }}
            className="absolute bottom-12 right-12 glass-card rounded-2xl p-6"
          >
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-emerald-500/10 flex items-center justify-center">
                <TrendingUp className="w-6 h-6 text-emerald-400" />
              </div>
              <div>
                <p className="text-sm text-gray-500">Portfolio Today</p>
                <p className="text-2xl font-bold text-white">+2.47%</p>
                <p className="text-sm text-emerald-400">€3,240.50</p>
              </div>
            </div>
          </motion.div>
        </div>

        {/* Glow Effects */}
        <div className="absolute top-1/4 right-1/4 w-96 h-96 bg-blue-500/10 rounded-full blur-3xl" />
        <div className="absolute bottom-1/4 left-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-3xl" />
      </div>
    </div>
  );
}
