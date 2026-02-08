"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { TrendingUp, TrendingDown, Loader2, Package } from "lucide-react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api";

interface Position {
  id: string;
  ticker: string;
  entry_date: string;
  entry_price: number;
  current_price: number;
  shares: number;
  pnl: number;
  pnl_percent: number;
  weight: number;
}

export default function PositionsPage() {
  const [positions, setPositions] = useState<Position[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [portfolio, setPortfolio] = useState({
    total_pnl: 0,
    total_value: 0,
  });

  useEffect(() => {
    fetchPositions();
  }, []);

  const fetchPositions = async () => {
    try {
      const res = await fetch(`${API_URL}/positions`);
      if (!res.ok) throw new Error("Failed to fetch");
      const data = await res.json();
      setPositions(data.data || []);
      
      // Calculate totals
      const totalPnl = (data.data || []).reduce((sum: number, p: Position) => sum + p.pnl, 0);
      const totalValue = (data.data || []).reduce((sum: number, p: Position) => 
        sum + (p.current_price * p.shares), 0);
      setPortfolio({ total_pnl: totalPnl, total_value: totalValue });
    } catch {
      // Fallback data for development
      const fallbackPositions = [
        {
          id: "1",
          ticker: "AAPL",
          entry_date: "2026-01-15",
          entry_price: 185.50,
          current_price: 195.25,
          shares: 100,
          pnl: 975.00,
          pnl_percent: 0.0526,
          weight: 0.156,
        },
        {
          id: "2",
          ticker: "MSFT",
          entry_date: "2026-01-10",
          entry_price: 380.00,
          current_price: 415.50,
          shares: 50,
          pnl: 1775.00,
          pnl_percent: 0.0934,
          weight: 0.166,
        },
        {
          id: "3",
          ticker: "GOOGL",
          entry_date: "2026-01-20",
          entry_price: 142.00,
          current_price: 138.75,
          shares: 75,
          pnl: -243.75,
          pnl_percent: -0.0229,
          weight: 0.083,
        },
        {
          id: "4",
          ticker: "NVDA",
          entry_date: "2026-01-05",
          entry_price: 520.00,
          current_price: 675.00,
          shares: 30,
          pnl: 4650.00,
          pnl_percent: 0.2981,
          weight: 0.162,
        },
        {
          id: "5",
          ticker: "AMZN",
          entry_date: "2026-01-25",
          entry_price: 155.00,
          current_price: 152.30,
          shares: 60,
          pnl: -162.00,
          pnl_percent: -0.0174,
          weight: 0.073,
        },
      ];
      setPositions(fallbackPositions);
      
      const totalPnl = fallbackPositions.reduce((sum, p) => sum + p.pnl, 0);
      const totalValue = fallbackPositions.reduce((sum, p) => 
        sum + (p.current_price * p.shares), 0);
      setPortfolio({ total_pnl: totalPnl, total_value: totalValue });
    } finally {
      setIsLoading(false);
    }
  };

  const totalPositions = positions.length;
  const winningPositions = positions.filter(p => p.pnl > 0).length;
  const losingPositions = positions.filter(p => p.pnl < 0).length;
  const winRate = totalPositions > 0 ? (winningPositions / totalPositions) * 100 : 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-slate-900">Positions</h1>
        <Badge variant="outline" className="px-3 py-1">
          {totalPositions} Active Positions
        </Badge>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Total P&L
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${portfolio.total_pnl >= 0 ? "text-green-600" : "text-red-600"}`}>
              {portfolio.total_pnl >= 0 ? "+" : ""}
              €{portfolio.total_pnl.toLocaleString("en-US", { minimumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Portfolio Value
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-slate-900">
              €{portfolio.total_value.toLocaleString("en-US", { minimumFractionDigits: 2 })}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Win Rate
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-slate-900">
              {winRate.toFixed(0)}%
            </div>
            <p className="text-xs text-slate-500 mt-1">
              {winningPositions} winning / {losingPositions} losing
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-slate-600">
              Positions
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-4">
              <div className="text-center">
                <div className="text-xl font-bold text-green-600">{winningPositions}</div>
                <div className="text-xs text-slate-500">Winning</div>
              </div>
              <div className="text-slate-300">|</div>
              <div className="text-center">
                <div className="text-xl font-bold text-red-600">{losingPositions}</div>
                <div className="text-xs text-slate-500">Losing</div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Positions Table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg flex items-center gap-2">
            <Package className="w-5 h-5" />
            Current Holdings
          </CardTitle>
        </CardHeader>
        <CardContent>
          {positions.length === 0 ? (
            <div className="text-center py-12">
              <p className="text-slate-500">No active positions</p>
              <p className="text-sm text-slate-400 mt-1">
                Positions will appear here after you confirm trade proposals
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Ticker</TableHead>
                    <TableHead>Entry Date</TableHead>
                    <TableHead className="text-right">Entry Price</TableHead>
                    <TableHead className="text-right">Current</TableHead>
                    <TableHead className="text-right">Shares</TableHead>
                    <TableHead className="text-right">P&L</TableHead>
                    <TableHead className="text-right">P&L %</TableHead>
                    <TableHead className="text-right">% NAV</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {positions.map((position) => (
                    <TableRow key={position.id}>
                      <TableCell className="font-medium">{position.ticker}</TableCell>
                      <TableCell className="text-slate-600">
                        {new Date(position.entry_date).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-right">
                        €{position.entry_price.toFixed(2)}
                      </TableCell>
                      <TableCell className="text-right font-medium">
                        €{position.current_price.toFixed(2)}
                      </TableCell>
                      <TableCell className="text-right">
                        {position.shares}
                      </TableCell>
                      <TableCell className={`text-right font-medium ${position.pnl >= 0 ? "text-green-600" : "text-red-600"}`}>
                        {position.pnl >= 0 ? "+" : ""}
                        €{position.pnl.toLocaleString("en-US", { minimumFractionDigits: 2 })}
                      </TableCell>
                      <TableCell className={`text-right ${position.pnl_percent >= 0 ? "text-green-600" : "text-red-600"}`}>
                        <div className="flex items-center justify-end gap-1">
                          {position.pnl_percent >= 0 ? (
                            <TrendingUp className="w-4 h-4" />
                          ) : (
                            <TrendingDown className="w-4 h-4" />
                          )}
                          {(position.pnl_percent * 100).toFixed(2)}%
                        </div>
                      </TableCell>
                      <TableCell className="text-right">
                        {(position.weight * 100).toFixed(1)}%
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
