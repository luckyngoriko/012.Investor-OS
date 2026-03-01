"use client";

import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import {
  fetchCurrentUser,
  loginWithPassword,
  logoutCurrentSession,
  refreshAuthSession,
  type AuthUser as User,
  type UserRole,
} from "@/lib/auth-api";

export type { UserRole };
export type { User };

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

const STORAGE_KEYS = {
  user: "auth.user",
  accessToken: "auth.accessToken",
  refreshToken: "auth.refreshToken",
} as const;
const SESSION_COOKIE_NAME = "investor_os_session";

function setSessionCookie() {
  document.cookie = `${SESSION_COOKIE_NAME}=1; Path=/; Max-Age=86400; SameSite=Lax`;
}

function clearSessionCookie() {
  document.cookie = `${SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; SameSite=Lax`;
}

function persistSession(user: User, accessToken: string, refreshToken: string) {
  localStorage.setItem(STORAGE_KEYS.user, JSON.stringify(user));
  localStorage.setItem(STORAGE_KEYS.accessToken, accessToken);
  localStorage.setItem(STORAGE_KEYS.refreshToken, refreshToken);
  setSessionCookie();
}

function clearSessionStorage() {
  localStorage.removeItem(STORAGE_KEYS.user);
  localStorage.removeItem(STORAGE_KEYS.accessToken);
  localStorage.removeItem(STORAGE_KEYS.refreshToken);
  clearSessionCookie();
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let mounted = true;

    const restoreSession = async () => {
      const accessToken = localStorage.getItem(STORAGE_KEYS.accessToken);
      const refreshToken = localStorage.getItem(STORAGE_KEYS.refreshToken);
      const storedUser = localStorage.getItem(STORAGE_KEYS.user);

      if (!accessToken || !refreshToken) {
        clearSessionStorage();
        if (mounted) {
          setUser(null);
          setIsLoading(false);
        }
        return;
      }

      try {
        const profile = await fetchCurrentUser(accessToken);
        persistSession(profile, accessToken, refreshToken);
        if (mounted) {
          setUser(profile);
        }
      } catch {
        try {
          const refreshed = await refreshAuthSession(refreshToken);
          const profile = await fetchCurrentUser(refreshed.access_token);
          persistSession(profile, refreshed.access_token, refreshed.refresh_token);
          if (mounted) {
            setUser(profile);
          }
        } catch {
          if (storedUser) {
            try {
              JSON.parse(storedUser);
            } catch {
              // Ignore malformed cached user payload.
            }
          }
          clearSessionStorage();
          if (mounted) {
            setUser(null);
          }
        }
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    void restoreSession();

    return () => {
      mounted = false;
    };
  }, []);

  const login = async (email: string, password: string) => {
    const session = await loginWithPassword(email, password);
    persistSession(session.user, session.access_token, session.refresh_token);
    setUser(session.user);
  };

  const logout = () => {
    const accessToken = localStorage.getItem(STORAGE_KEYS.accessToken);
    const refreshToken = localStorage.getItem(STORAGE_KEYS.refreshToken);
    void logoutCurrentSession(accessToken, refreshToken);

    setUser(null);
    clearSessionStorage();
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
