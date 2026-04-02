"use client";

import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Store,
  TrendingUp,
  Users,
  DollarSign,
  BarChart3,
  Loader2,
  AlertTriangle,
  Copy,
  Star,
} from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────

interface StrategyListing {
  id: string;
  creator_id: string;
  strategy_id: string;
  name: string;
  description: string | null;
  performance: {
    sharpe?: number;
    total_return_pct?: number;
    win_rate?: number;
  };
  price_monthly_usd: number;
  subscribers_count: number;
  is_public: boolean;
  created_at: string;
}

// ── Page Component ─────────────────────────────────────────────────

export default function MarketplacePage() {
  const [listings, setListings] = useState<StrategyListing[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [subscribing, setSubscribing] = useState<string | null>(null);

  useEffect(() => {
    fetchListings();
  }, []);

  const fetchListings = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/marketplace");
      const data = await res.json();
      if (data.success) {
        setListings(data.data);
      } else {
        setError(data.error?.message || "Failed to load marketplace");
      }
    } catch {
      setError("Failed to connect to marketplace API");
    } finally {
      setLoading(false);
    }
  };

  const handleSubscribe = async (listingId: string) => {
    setSubscribing(listingId);
    try {
      const res = await fetch(`/api/marketplace/${listingId}/subscribe`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });
      const data = await res.json();
      if (data.success) {
        setListings((prev) =>
          prev.map((l) =>
            l.id === listingId
              ? { ...l, subscribers_count: l.subscribers_count + 1 }
              : l,
          ),
        );
      }
    } catch {
      // silent fail for subscribe
    } finally {
      setSubscribing(null);
    }
  };

  return (
    <div className="min-h-screen text-slate-100">
      <div className="p-6 space-y-6">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <div className="flex items-center gap-3 mb-2">
            <Store className="h-8 w-8 text-purple-400" />
            <h1 className="text-3xl font-bold">Copy Trading Marketplace</h1>
          </div>
          <p className="text-slate-400">
            Browse and subscribe to strategies created by top traders
          </p>
        </motion.div>

        {/* Loading */}
        {loading && (
          <div className="flex items-center justify-center py-20">
            <Loader2 className="h-8 w-8 animate-spin text-purple-400" />
            <span className="ml-3 text-slate-400">Loading marketplace...</span>
          </div>
        )}

        {/* Error */}
        {error && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-center gap-3"
          >
            <AlertTriangle className="h-5 w-5 text-red-400" />
            <span className="text-red-300">{error}</span>
          </motion.div>
        )}

        {/* Empty State */}
        {!loading && !error && listings.length === 0 && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="text-center py-20"
          >
            <Store className="h-16 w-16 text-slate-600 mx-auto mb-4" />
            <h2 className="text-xl font-semibold text-slate-400 mb-2">
              No strategies listed yet
            </h2>
            <p className="text-slate-500">
              Be the first to publish a strategy to the marketplace
            </p>
          </motion.div>
        )}

        {/* Strategy Grid */}
        {!loading && listings.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {listings.map((listing, index) => (
              <motion.div
                key={listing.id}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.05 }}
                className="bg-gray-800 border border-gray-700 rounded-xl p-6 hover:border-purple-500/50 transition-colors"
              >
                {/* Card Header */}
                <div className="flex items-start justify-between mb-4">
                  <div className="flex-1 min-w-0">
                    <h3 className="text-lg font-semibold text-slate-100 truncate">
                      {listing.name}
                    </h3>
                    <p className="text-sm text-slate-400 mt-1 line-clamp-2">
                      {listing.description || "No description provided"}
                    </p>
                  </div>
                  <div className="flex items-center gap-1 ml-3">
                    <Star className="h-4 w-4 text-yellow-400" />
                    <span className="text-sm text-slate-300">
                      {listing.subscribers_count}
                    </span>
                  </div>
                </div>

                {/* Performance Stats */}
                <div className="grid grid-cols-3 gap-3 mb-4">
                  <div className="bg-gray-900/50 rounded-lg p-3 text-center">
                    <TrendingUp className="h-4 w-4 text-green-400 mx-auto mb-1" />
                    <div className="text-sm font-medium text-slate-200">
                      {listing.performance?.total_return_pct != null
                        ? `${listing.performance.total_return_pct.toFixed(1)}%`
                        : "N/A"}
                    </div>
                    <div className="text-xs text-slate-500">Return</div>
                  </div>
                  <div className="bg-gray-900/50 rounded-lg p-3 text-center">
                    <BarChart3 className="h-4 w-4 text-blue-400 mx-auto mb-1" />
                    <div className="text-sm font-medium text-slate-200">
                      {listing.performance?.sharpe != null
                        ? listing.performance.sharpe.toFixed(2)
                        : "N/A"}
                    </div>
                    <div className="text-xs text-slate-500">Sharpe</div>
                  </div>
                  <div className="bg-gray-900/50 rounded-lg p-3 text-center">
                    <Users className="h-4 w-4 text-purple-400 mx-auto mb-1" />
                    <div className="text-sm font-medium text-slate-200">
                      {listing.performance?.win_rate != null
                        ? `${(listing.performance.win_rate * 100).toFixed(0)}%`
                        : "N/A"}
                    </div>
                    <div className="text-xs text-slate-500">Win Rate</div>
                  </div>
                </div>

                {/* Price + Subscribe */}
                <div className="flex items-center justify-between pt-4 border-t border-gray-700">
                  <div className="flex items-center gap-1">
                    <DollarSign className="h-4 w-4 text-emerald-400" />
                    <span className="text-lg font-bold text-slate-100">
                      {listing.price_monthly_usd === 0
                        ? "Free"
                        : `$${listing.price_monthly_usd}/mo`}
                    </span>
                  </div>
                  <button
                    onClick={() => handleSubscribe(listing.id)}
                    disabled={subscribing === listing.id}
                    className="flex items-center gap-2 px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:bg-purple-800 rounded-lg text-sm font-medium transition-colors"
                  >
                    {subscribing === listing.id ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                    Copy Trade
                  </button>
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
