"use client";

import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  TrendingUp,
  TrendingDown,
  Search,
  Filter,
  ArrowUpDown,
  MoreHorizontal,
  Download,
  Plus,
  Wallet,
  PieChart,
  Target,
  AlertTriangle,
} from "lucide-react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";

interface Position {
  id: string;
  ticker: string;
  company: string;
  shares: number;
  avgPrice: number;
  currentPrice: number;
  marketValue: number;
  pnl: number;
  pnlPercent: number;
  weight: number;
  sector: string;
  dayChange: number;
  dayChangePercent: number;
}

export default function PositionsPage() {
  const [positions, setPositions] = useState<Position[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState<"pnl" | "value" | "weight">("pnl");
  const [filterSector, setFilterSector] = useState<string>("all");

  useEffect(() => {
    const initialPositions: Position[] = [
      {
        id: "1",
        ticker: "AAPL",
        company: "Apple Inc.",
        shares: 100,
        avgPrice: 185.5,
        currentPrice: 195.25,
        marketValue: 19525,
        pnl: 975,
        pnlPercent: 5.26,
        weight: 15.6,
        sector: "Technology",
        dayChange: 2.35,
        dayChangePercent: 1.22,
      },
      {
        id: "2",
        ticker: "MSFT",
        company: "Microsoft Corp.",
        shares: 50,
        avgPrice: 380.0,
        currentPrice: 415.5,
        marketValue: 20775,
        pnl: 1775,
        pnlPercent: 9.34,
        weight: 16.6,
        sector: "Technology",
        dayChange: 4.2,
        dayChangePercent: 1.02,
      },
      {
        id: "3",
        ticker: "NVDA",
        company: "NVIDIA Corp.",
        shares: 30,
        avgPrice: 520.0,
        currentPrice: 675.0,
        marketValue: 20250,
        pnl: 4650,
        pnlPercent: 29.81,
        weight: 16.2,
        sector: "Technology",
        dayChange: 12.5,
        dayChangePercent: 1.89,
      },
      {
        id: "4",
        ticker: "GOOGL",
        company: "Alphabet Inc.",
        shares: 75,
        avgPrice: 142.0,
        currentPrice: 138.75,
        marketValue: 10406.25,
        pnl: -243.75,
        pnlPercent: -2.29,
        weight: 8.3,
        sector: "Technology",
        dayChange: -1.25,
        dayChangePercent: -0.89,
      },
      {
        id: "5",
        ticker: "AMZN",
        company: "Amazon.com Inc.",
        shares: 60,
        avgPrice: 155.0,
        currentPrice: 152.3,
        marketValue: 9138,
        pnl: -162,
        pnlPercent: -1.74,
        weight: 7.3,
        sector: "Consumer",
        dayChange: -0.85,
        dayChangePercent: -0.55,
      },
      {
        id: "6",
        ticker: "TSLA",
        company: "Tesla Inc.",
        shares: 40,
        avgPrice: 240.0,
        currentPrice: 248.5,
        marketValue: 9940,
        pnl: 340,
        pnlPercent: 1.42,
        weight: 7.9,
        sector: "Consumer",
        dayChange: 5.4,
        dayChangePercent: 2.22,
      },
      {
        id: "7",
        ticker: "JPM",
        company: "JPMorgan Chase",
        shares: 25,
        avgPrice: 175.0,
        currentPrice: 182.75,
        marketValue: 4568.75,
        pnl: 193.75,
        pnlPercent: 4.43,
        weight: 3.7,
        sector: "Finance",
        dayChange: 1.85,
        dayChangePercent: 1.02,
      },
    ];
    setPositions(initialPositions);

    // Real-time price updates
    const interval = setInterval(() => {
      setPositions((prev) =>
        prev.map((pos) => {
          const change = (Math.random() - 0.5) * 0.02;
          const newPrice = pos.currentPrice * (1 + change);
          const newPnl = (newPrice - pos.avgPrice) * pos.shares;
          const newPnlPercent = ((newPrice - pos.avgPrice) / pos.avgPrice) * 100;
          return {
            ...pos,
            currentPrice: newPrice,
            marketValue: newPrice * pos.shares,
            pnl: newPnl,
            pnlPercent: newPnlPercent,
          };
        })
      );
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const filteredPositions = positions
    .filter(
      (p) =>
        (filterSector === "all" || p.sector === filterSector) &&
        (p.ticker.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.company.toLowerCase().includes(searchQuery.toLowerCase()))
    )
    .sort((a, b) => {
      if (sortBy === "pnl") return b.pnl - a.pnl;
      if (sortBy === "value") return b.marketValue - a.marketValue;
      if (sortBy === "weight") return b.weight - a.weight;
      return 0;
    });

  const totalValue = positions.reduce((sum, p) => sum + p.marketValue, 0);
  const totalPnl = positions.reduce((sum, p) => sum + p.pnl, 0);
  const totalDayChange = positions.reduce((sum, p) => sum + p.dayChange * p.shares, 0);
  const winningPositions = positions.filter((p) => p.pnl > 0).length;

  const sectorData = [
    { name: "Technology", value: 55, pnl: 6156 },
    { name: "Consumer", value: 15, pnl: 178 },
    { name: "Finance", value: 10, pnl: 194 },
    { name: "Healthcare", value: 12, pnl: 420 },
    { name: "Energy", value: 8, pnl: -120 },
  ];

  return (
    <div className="min-h-screen bg-[#0a0f1c] text-white p-6">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-3xl font-bold text-white mb-1">Portfolio Positions</h1>
            <p className="text-gray-500">Manage your active investments and track performance</p>
          </div>
          <div className="flex items-center gap-3">
            <button className="flex items-center gap-2 px-4 py-2 rounded-xl bg-gray-800 text-gray-300 hover:bg-gray-700 transition-colors">
              <Download className="w-4 h-4" />
              <span className="hidden md:inline">Export</span>
            </button>
            <button className="flex items-center gap-2 px-4 py-2 rounded-xl bg-blue-600 text-white hover:bg-blue-500 transition-colors shadow-lg shadow-blue-600/30">
              <Plus className="w-4 h-4" />
              <span className="hidden md:inline">New Trade</span>
            </button>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="stat-card"
          >
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-400 text-sm">Total Value</span>
              <div className="p-2 rounded-lg bg-blue-500/10">
                <Wallet className="w-4 h-4 text-blue-400" />
              </div>
            </div>
            <div className="stat-value">
              €{totalValue.toLocaleString("en-US", { minimumFractionDigits: 2 })}
            </div>
            <div className={`stat-change ${totalDayChange >= 0 ? "positive" : "negative"} mt-2`}>
              {totalDayChange >= 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
              <span>Today: {totalDayChange >= 0 ? "+" : ""}€{Math.abs(totalDayChange).toFixed(2)}</span>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="stat-card"
          >
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-400 text-sm">Total P&L</span>
              <div className="p-2 rounded-lg bg-emerald-500/10">
                <PieChart className="w-4 h-4 text-emerald-400" />
              </div>
            </div>
            <div className={`stat-value ${totalPnl >= 0 ? "text-emerald-400" : "text-red-400"}`}>
              {totalPnl >= 0 ? "+" : ""}€{totalPnl.toLocaleString("en-US", { minimumFractionDigits: 2 })}
            </div>
            <div className={`stat-change ${totalPnl >= 0 ? "positive" : "negative"} mt-2`}>
              {totalPnl >= 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
              <span>All time</span>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="stat-card"
          >
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-400 text-sm">Win Rate</span>
              <div className="p-2 rounded-lg bg-purple-500/10">
                <Target className="w-4 h-4 text-purple-400" />
              </div>
            </div>
            <div className="stat-value">
              {((winningPositions / positions.length) * 100).toFixed(0)}%
            </div>
            <div className="stat-label mt-2">
              {winningPositions} of {positions.length} positions profitable
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
            className="stat-card"
          >
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-400 text-sm">Risk Alert</span>
              <div className="p-2 rounded-lg bg-amber-500/10">
                <AlertTriangle className="w-4 h-4 text-amber-400" />
              </div>
            </div>
            <div className="stat-value text-amber-400">Medium</div>
            <div className="stat-label mt-2">Concentration in Tech: 55%</div>
          </motion.div>
        </div>
      </div>

      {/* Charts & Table Section */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Sector Performance Chart */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="glass-card rounded-2xl p-6"
        >
          <h3 className="text-lg font-semibold text-white mb-6">Sector Performance</h3>
          <div className="h-[250px]">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={sectorData} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" horizontal={false} />
                <XAxis type="number" stroke="#4b5563" fontSize={12} />
                <YAxis dataKey="name" type="category" stroke="#9ca3af" fontSize={11} width={80} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#1f2937",
                    border: "1px solid #374151",
                    borderRadius: "12px",
                  }}
                  formatter={(value) => [`€${Number(value).toLocaleString()}`, "P&L"]}
                />
                <Bar dataKey="pnl" radius={[0, 4, 4, 0]}>
                  {sectorData.map((entry, index) => (
                    <Cell
                      key={`cell-${index}`}
                      fill={entry.pnl >= 0 ? "#10b981" : "#ef4444"}
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </motion.div>

        {/* Positions Table */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="lg:col-span-2 glass-card rounded-2xl p-6"
        >
          {/* Filters */}
          <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 mb-6">
            <div className="flex items-center gap-3">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
                <input
                  type="text"
                  placeholder="Search positions..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10 pr-4 py-2 bg-gray-800 border border-gray-700 rounded-xl text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 w-64"
                />
              </div>
              <select
                value={filterSector}
                onChange={(e) => setFilterSector(e.target.value)}
                className="px-4 py-2 bg-gray-800 border border-gray-700 rounded-xl text-sm text-white focus:outline-none focus:border-blue-500"
              >
                <option value="all">All Sectors</option>
                <option value="Technology">Technology</option>
                <option value="Consumer">Consumer</option>
                <option value="Finance">Finance</option>
                <option value="Healthcare">Healthcare</option>
              </select>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-500">Sort by:</span>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="px-3 py-2 bg-gray-800 border border-gray-700 rounded-xl text-sm text-white focus:outline-none focus:border-blue-500"
              >
                <option value="pnl">P&L</option>
                <option value="value">Market Value</option>
                <option value="weight">Weight</option>
              </select>
            </div>
          </div>

          {/* Table */}
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-gray-800">
                  <th className="text-left py-3 px-4 text-sm font-medium text-gray-400">Asset</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">Shares</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">Avg Price</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">Current</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">Market Value</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">P&L</th>
                  <th className="text-right py-3 px-4 text-sm font-medium text-gray-400">Weight</th>
                  <th className="text-center py-3 px-4 text-sm font-medium text-gray-400">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredPositions.map((position, index) => (
                  <motion.tr
                    key={position.id}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: index * 0.05 }}
                    className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors"
                  >
                    <td className="py-4 px-4">
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500/20 to-purple-500/20 flex items-center justify-center">
                          <span className="font-bold text-blue-400">{position.ticker[0]}</span>
                        </div>
                        <div>
                          <p className="font-semibold text-white">{position.ticker}</p>
                          <p className="text-xs text-gray-500">{position.sector}</p>
                        </div>
                      </div>
                    </td>
                    <td className="py-4 px-4 text-right text-white">{position.shares}</td>
                    <td className="py-4 px-4 text-right text-gray-400">
                      €{position.avgPrice.toFixed(2)}
                    </td>
                    <td className="py-4 px-4 text-right">
                      <div className="flex flex-col items-end">
                        <span className="text-white">€{position.currentPrice.toFixed(2)}</span>
                        <span
                          className={`text-xs flex items-center gap-1 ${
                            position.dayChange >= 0 ? "text-emerald-400" : "text-red-400"
                          }`}
                        >
                          {position.dayChange >= 0 ? "+" : ""}
                          {position.dayChangePercent.toFixed(2)}%
                        </span>
                      </div>
                    </td>
                    <td className="py-4 px-4 text-right text-white font-medium">
                      €{position.marketValue.toLocaleString("en-US", { minimumFractionDigits: 2 })}
                    </td>
                    <td className="py-4 px-4 text-right">
                      <div className="flex flex-col items-end">
                        <span
                          className={`font-medium ${
                            position.pnl >= 0 ? "text-emerald-400" : "text-red-400"
                          }`}
                        >
                          {position.pnl >= 0 ? "+" : ""}€
                          {Math.abs(position.pnl).toFixed(2)}
                        </span>
                        <span
                          className={`text-xs ${
                            position.pnlPercent >= 0 ? "text-emerald-400" : "text-red-400"
                          }`}
                        >
                          {position.pnlPercent >= 0 ? "+" : ""}
                          {position.pnlPercent.toFixed(2)}%
                        </span>
                      </div>
                    </td>
                    <td className="py-4 px-4 text-right">
                      <div className="flex items-center justify-end gap-2">
                        <div className="h-1.5 w-16 bg-gray-700 rounded-full overflow-hidden">
                          <div
                            className="h-full bg-blue-500 rounded-full"
                            style={{ width: `${position.weight}%` }}
                          />
                        </div>
                        <span className="text-sm text-gray-400 w-12">{position.weight.toFixed(1)}%</span>
                      </div>
                    </td>
                    <td className="py-4 px-4 text-center">
                      <button className="p-2 rounded-lg hover:bg-gray-700 text-gray-400 transition-colors">
                        <MoreHorizontal className="w-4 h-4" />
                      </button>
                    </td>
                  </motion.tr>
                ))}
              </tbody>
            </table>
          </div>

          {filteredPositions.length === 0 && (
            <div className="text-center py-12">
              <p className="text-gray-500">No positions found matching your criteria</p>
            </div>
          )}
        </motion.div>
      </div>
    </div>
  );
}
