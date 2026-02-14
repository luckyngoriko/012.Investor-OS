"use client";

import { useState, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Bell,
  X,
  Check,
  CheckCheck,
  AlertTriangle,
  Info,
  Sparkles,
  TrendingUp,
  TrendingDown,
  DollarSign,
  Clock,
  Trash2,
  Settings,
  Volume2,
  VolumeX,
} from "lucide-react";

// Notification types
export type NotificationType = "info" | "success" | "warning" | "error" | "trade" | "ai";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
  action?: {
    label: string;
    onClick: () => void;
  };
  persistent?: boolean;
}

// Notification store (could be replaced with global state management)
let notificationsStore: Notification[] = [];
let listeners: ((notifications: Notification[]) => void)[] = [];

function notifyListeners() {
  listeners.forEach((listener) => listener([...notificationsStore]));
}

export function addNotification(notification: Omit<Notification, "id" | "timestamp" | "read">) {
  const newNotification: Notification = {
    ...notification,
    id: Math.random().toString(36).substring(7),
    timestamp: new Date(),
    read: false,
  };
  notificationsStore = [newNotification, ...notificationsStore];
  notifyListeners();
  return newNotification.id;
}

export function removeNotification(id: string) {
  notificationsStore = notificationsStore.filter((n) => n.id !== id);
  notifyListeners();
}

export function markAsRead(id: string) {
  notificationsStore = notificationsStore.map((n) =>
    n.id === id ? { ...n, read: true } : n
  );
  notifyListeners();
}

export function markAllAsRead() {
  notificationsStore = notificationsStore.map((n) => ({ ...n, read: true }));
  notifyListeners();
}

export function clearAllNotifications() {
  notificationsStore = [];
  notifyListeners();
}

export function useNotifications() {
  const [notifications, setNotifications] = useState<Notification[]>([]);

  useEffect(() => {
    listeners.push(setNotifications);
    setNotifications([...notificationsStore]);
    return () => {
      listeners = listeners.filter((l) => l !== setNotifications);
    };
  }, []);

  return {
    notifications,
    unreadCount: notifications.filter((n) => !n.read).length,
    addNotification,
    removeNotification,
    markAsRead,
    markAllAsRead,
    clearAllNotifications,
  };
}

// Toast notification component
export function ToastContainer() {
  const [toasts, setToasts] = useState<Notification[]>([]);
  const { notifications } = useNotifications();

  useEffect(() => {
    // Show new notifications as toasts
    const newNotifications = notifications.filter(
      (n) => !n.read && !n.persistent && !toasts.find((t) => t.id === n.id)
    );

    if (newNotifications.length > 0) {
      setToasts((prev) => [...newNotifications, ...prev].slice(0, 5));

      // Auto dismiss after 5 seconds
      newNotifications.forEach((n) => {
        setTimeout(() => {
          setToasts((prev) => prev.filter((t) => t.id !== n.id));
          markAsRead(n.id);
        }, 5000);
      });
    }
  }, [notifications, toasts]);

  const removeToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
    markAsRead(id);
  };

  return (
    <div className="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      <AnimatePresence>
        {toasts.map((toast) => (
          <Toast key={toast.id} notification={toast} onClose={removeToast} />
        ))}
      </AnimatePresence>
    </div>
  );
}

function Toast({
  notification,
  onClose,
}: {
  notification: Notification;
  onClose: (id: string) => void;
}) {
  const icons = {
    info: Info,
    success: Check,
    warning: AlertTriangle,
    error: AlertTriangle,
    trade: DollarSign,
    ai: Sparkles,
  };

  const colors = {
    info: "bg-blue-500/10 border-blue-500/30 text-blue-400",
    success: "bg-emerald-500/10 border-emerald-500/30 text-emerald-400",
    warning: "bg-amber-500/10 border-amber-500/30 text-amber-400",
    error: "bg-rose-500/10 border-rose-500/30 text-rose-400",
    trade: "bg-purple-500/10 border-purple-500/30 text-purple-400",
    ai: "bg-cyan-500/10 border-cyan-500/30 text-cyan-400",
  };

  const Icon = icons[notification.type];

  return (
    <motion.div
      initial={{ opacity: 0, x: 50, scale: 0.9 }}
      animate={{ opacity: 1, x: 0, scale: 1 }}
      exit={{ opacity: 0, x: 50, scale: 0.9 }}
      className={`pointer-events-auto w-80 p-4 rounded-xl border shadow-lg
        backdrop-blur-lg ${colors[notification.type]}`}
    >
      <div className="flex items-start gap-3">
        <Icon className="w-5 h-5 flex-shrink-0 mt-0.5" />
        <div className="flex-1 min-w-0">
          <h4 className="font-medium text-sm text-white">{notification.title}</h4>
          <p className="text-sm text-gray-300 mt-1">{notification.message}</p>
          {notification.action && (
            <button
              onClick={() => {
                notification.action?.onClick();
                onClose(notification.id);
              }}
              className="mt-2 text-sm font-medium underline hover:no-underline"
            >
              {notification.action.label}
            </button>
          )}
        </div>
        <button
          onClick={() => onClose(notification.id)}
          className="text-gray-400 hover:text-white transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </motion.div>
  );
}

// Main notification center component
export function NotificationCenter() {
  const [isOpen, setIsOpen] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const { notifications, unreadCount, markAsRead, markAllAsRead, clearAllNotifications } =
    useNotifications();

  const getIcon = (type: NotificationType) => {
    switch (type) {
      case "success":
        return <Check className="w-4 h-4 text-emerald-400" />;
      case "warning":
        return <AlertTriangle className="w-4 h-4 text-amber-400" />;
      case "error":
        return <AlertTriangle className="w-4 h-4 text-rose-400" />;
      case "trade":
        return <DollarSign className="w-4 h-4 text-purple-400" />;
      case "ai":
        return <Sparkles className="w-4 h-4 text-cyan-400" />;
      default:
        return <Info className="w-4 h-4 text-blue-400" />;
    }
  };

  const getBgColor = (type: NotificationType, read: boolean) => {
    if (read) return "bg-transparent";
    switch (type) {
      case "success":
        return "bg-emerald-500/5";
      case "warning":
        return "bg-amber-500/5";
      case "error":
        return "bg-rose-500/5";
      case "trade":
        return "bg-purple-500/5";
      case "ai":
        return "bg-cyan-500/5";
      default:
        return "bg-blue-500/5";
    }
  };

  const formatTime = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - new Date(date).getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return "Just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return `${days}d ago`;
  };

  return (
    <>
      {/* Bell Button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="relative w-10 h-10 rounded-xl bg-gray-800/50 hover:bg-gray-700/50 
          border border-gray-700/50 flex items-center justify-center
          text-gray-400 hover:text-white transition-colors"
      >
        <Bell className="w-4 h-4" />
        {unreadCount > 0 && (
          <span className="absolute -top-1 -right-1 w-5 h-5 bg-rose-500 text-white 
            text-xs font-bold rounded-full flex items-center justify-center">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </button>

      <AnimatePresence>
        {isOpen && (
          <>
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 z-40"
              onClick={() => setIsOpen(false)}
            />

            {/* Notification Panel */}
            <motion.div
              initial={{ opacity: 0, y: 10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 10, scale: 0.95 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full right-0 mt-2 w-96 z-50"
            >
              <div className="glass-card rounded-2xl border border-gray-700/50 shadow-2xl overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b border-gray-800/50">
                  <div className="flex items-center gap-2">
                    <Bell className="w-4 h-4 text-gray-400" />
                    <span className="font-medium text-white">Notifications</span>
                    {unreadCount > 0 && (
                      <span className="px-2 py-0.5 text-xs bg-rose-500 text-white rounded-full">
                        {unreadCount}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => setSoundEnabled(!soundEnabled)}
                      className="p-2 text-gray-400 hover:text-white rounded-lg 
                        hover:bg-gray-800/50 transition-colors"
                      title={soundEnabled ? "Mute notifications" : "Unmute notifications"}
                    >
                      {soundEnabled ? (
                        <Volume2 className="w-4 h-4" />
                      ) : (
                        <VolumeX className="w-4 h-4" />
                      )}
                    </button>
                    {notifications.length > 0 && (
                      <>
                        <button
                          onClick={markAllAsRead}
                          className="p-2 text-gray-400 hover:text-white rounded-lg 
                            hover:bg-gray-800/50 transition-colors"
                          title="Mark all as read"
                        >
                          <CheckCheck className="w-4 h-4" />
                        </button>
                        <button
                          onClick={clearAllNotifications}
                          className="p-2 text-gray-400 hover:text-rose-400 rounded-lg 
                            hover:bg-rose-500/10 transition-colors"
                          title="Clear all"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </>
                    )}
                  </div>
                </div>

                {/* Notifications List */}
                <div className="max-h-[400px] overflow-y-auto">
                  {notifications.length === 0 ? (
                    <div className="px-4 py-12 text-center">
                      <Bell className="w-12 h-12 mx-auto mb-3 text-gray-600" />
                      <p className="text-gray-400">No notifications</p>
                      <p className="text-sm text-gray-500 mt-1">
                        You&apos;ll see updates here
                      </p>
                    </div>
                  ) : (
                    <div className="divide-y divide-gray-800/50">
                      {notifications.map((notification) => (
                        <div
                          key={notification.id}
                          onClick={() => markAsRead(notification.id)}
                          className={`flex items-start gap-3 px-4 py-3 cursor-pointer
                            transition-colors hover:bg-gray-800/30
                            ${getBgColor(notification.type, notification.read)}`}
                        >
                          <div className="flex-shrink-0 mt-0.5">
                            {getIcon(notification.type)}
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-start justify-between gap-2">
                              <h4 className={`font-medium text-sm ${notification.read ? "text-gray-400" : "text-white"}`}>
                                {notification.title}
                              </h4>
                              <span className="text-xs text-gray-500 flex-shrink-0">
                                {formatTime(notification.timestamp)}
                              </span>
                            </div>
                            <p className={`text-sm mt-0.5 ${notification.read ? "text-gray-500" : "text-gray-300"}`}>
                              {notification.message}
                            </p>
                            {notification.action && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  notification.action?.onClick();
                                }}
                                className="mt-2 text-sm font-medium text-blue-400 
                                  hover:text-blue-300 transition-colors"
                              >
                                {notification.action.label}
                              </button>
                            )}
                          </div>
                          {!notification.read && (
                            <div className="w-2 h-2 bg-blue-500 rounded-full flex-shrink-0 mt-2" />
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Footer */}
                <div className="px-4 py-3 bg-gray-800/30 border-t border-gray-800/50">
                  <button
                    onClick={() => setIsOpen(false)}
                    className="w-full py-2 text-sm text-gray-400 hover:text-white 
                      transition-colors"
                  >
                    Close
                  </button>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </>
  );
}

// Example usage helper
export function showTradeNotification(
  symbol: string,
  action: "buy" | "sell",
  quantity: number,
  price: number
) {
  addNotification({
    type: "trade",
    title: `Trade Executed: ${symbol}`,
    message: `${action === "buy" ? "Bought" : "Sold"} ${quantity} shares at $${price.toFixed(2)}`,
    action: {
      label: "View Position",
      onClick: () => {
        window.location.href = "/positions";
      },
    },
  });
}

export function showAIProposalNotification(symbol: string, confidence: number) {
  addNotification({
    type: "ai",
    title: `New AI Proposal: ${symbol}`,
    message: `AI suggests ${symbol} with ${confidence}% confidence`,
    action: {
      label: "Review",
      onClick: () => {
        window.location.href = "/proposals";
      },
    },
  });
}

export function showRiskAlert(message: string) {
  addNotification({
    type: "warning",
    title: "Risk Alert",
    message,
    persistent: true,
  });
}
