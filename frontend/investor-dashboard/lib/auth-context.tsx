"use client";

import { createContext, useContext, useState, useEffect, ReactNode } from "react";

export type UserRole = "admin" | "trader" | "viewer";

export interface User {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  avatar?: string;
  permissions: string[];
}

interface AuthContextType {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
  hasRole: (role: UserRole | UserRole[]) => boolean;
  hasPermission: (permission: string) => boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

// Mock users for demo
const MOCK_USERS: Record<string, User> = {
  "admin@investor-os.com": {
    id: "1",
    email: "admin@investor-os.com",
    name: "Admin User",
    role: "admin",
    avatar: "AU",
    permissions: ["*"], // All permissions
  },
  "trader@investor-os.com": {
    id: "2",
    email: "trader@investor-os.com",
    name: "John Trader",
    role: "trader",
    avatar: "JT",
    permissions: [
      "dashboard.read",
      "portfolio.read",
      "portfolio.trade",
      "positions.read",
      "proposals.read",
      "proposals.execute",
      "risk.read",
      "backtest.read",
      "backtest.run",
      "journal.read",
      "journal.write",
      "settings.read",
      "settings.update",
    ],
  },
  "viewer@investor-os.com": {
    id: "3",
    email: "viewer@investor-os.com",
    name: "View Only",
    role: "viewer",
    avatar: "VO",
    permissions: [
      "dashboard.read",
      "portfolio.read",
      "positions.read",
      "proposals.read",
      "risk.read",
      "journal.read",
    ],
  },
};

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Check for stored session on mount
  useEffect(() => {
    const storedUser = localStorage.getItem("user");
    if (storedUser) {
      try {
        setUser(JSON.parse(storedUser));
      } catch {
        localStorage.removeItem("user");
      }
    }
    setIsLoading(false);
  }, []);

  const login = async (email: string, password: string) => {
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const mockUser = MOCK_USERS[email.toLowerCase()];
    
    if (!mockUser || password !== "demo123") {
      throw new Error("Invalid credentials");
    }

    setUser(mockUser);
    localStorage.setItem("user", JSON.stringify(mockUser));
  };

  const logout = () => {
    setUser(null);
    localStorage.removeItem("user");
  };

  const hasRole = (role: UserRole | UserRole[]): boolean => {
    if (!user) return false;
    if (Array.isArray(role)) {
      return role.includes(user.role);
    }
    return user.role === role;
  };

  const hasPermission = (permission: string): boolean => {
    if (!user) return false;
    if (user.permissions.includes("*")) return true;
    return user.permissions.includes(permission);
  };

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: !!user,
        isLoading,
        login,
        logout,
        hasRole,
        hasPermission,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}

// Role-based component wrapper
export function RequireRole({ 
  roles, 
  children, 
  fallback = null 
}: { 
  roles: UserRole | UserRole[]; 
  children: ReactNode;
  fallback?: ReactNode;
}) {
  const { hasRole, isLoading } = useAuth();
  
  if (isLoading) return null;
  if (!hasRole(roles)) return fallback;
  
  return children;
}

// Permission-based component wrapper
export function RequirePermission({ 
  permission, 
  children, 
  fallback = null 
}: { 
  permission: string; 
  children: ReactNode;
  fallback?: ReactNode;
}) {
  const { hasPermission, isLoading } = useAuth();
  
  if (isLoading) return null;
  if (!hasPermission(permission)) return fallback;
  
  return children;
}
