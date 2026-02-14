/**
 * Signal Strength Chart
 * Historical conviction visualization
 */

import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  Area,
  ComposedChart,
} from 'recharts';
import { HistoricalPrediction } from '../types/hrm';

interface SignalStrengthChartProps {
  data: HistoricalPrediction[];
  height?: number;
}

export const SignalStrengthChart: React.FC<SignalStrengthChartProps> = ({
  data,
  height = 200,
}) => {
  // Format data for chart
  const chartData = [...data].reverse().map((item) => ({
    time: new Date(item.timestamp).toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
    }),
    conviction: item.conviction * 100,
    confidence: item.confidence * 100,
    strength: item.conviction * item.confidence * 100,
    regime: item.regime,
  }));

  // Custom tooltip
  const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div className="bg-white p-3 border border-gray-200 rounded-lg shadow-lg">
          <p className="text-sm text-gray-600">{label}</p>
          <p className="text-sm font-medium text-blue-600">
            Conviction: {data.conviction.toFixed(1)}%
          </p>
          <p className="text-sm font-medium text-purple-600">
            Confidence: {data.confidence.toFixed(1)}%
          </p>
          <p className="text-xs text-gray-500 mt-1">Regime: {data.regime}</p>
        </div>
      );
    }
    return null;
  };

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-48 text-gray-400">
        No historical data available
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height }}>
      <ResponsiveContainer>
        <ComposedChart data={chartData} margin={{ top: 5, right: 5, bottom: 5, left: 0 }}>
          <defs>
            <linearGradient id="convictionGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
              <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
            </linearGradient>
          </defs>
          
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          
          <XAxis
            dataKey="time"
            tick={{ fontSize: 10, fill: '#6b7280' }}
            tickMargin={5}
            interval="preserveStartEnd"
          />
          
          <YAxis
            domain={[0, 100]}
            tick={{ fontSize: 10, fill: '#6b7280' }}
            tickFormatter={(value) => `${value}%`}
          />
          
          <Tooltip content={<CustomTooltip />} />
          
          {/* Trading threshold lines */}
          <ReferenceLine
            y={80}
            stroke="#10b981"
            strokeDasharray="3 3"
            label={{ value: 'Strong Buy', position: 'right', fontSize: 10, fill: '#10b981' }}
          />
          <ReferenceLine
            y={60}
            stroke="#3b82f6"
            strokeDasharray="3 3"
            label={{ value: 'Buy', position: 'right', fontSize: 10, fill: '#3b82f6' }}
          />
          <ReferenceLine
            y={40}
            stroke="#f59e0b"
            strokeDasharray="3 3"
            label={{ value: 'Neutral', position: 'right', fontSize: 10, fill: '#f59e0b' }}
          />
          
          {/* Conviction area */}
          <Area
            type="monotone"
            dataKey="conviction"
            stroke="#3b82f6"
            strokeWidth={2}
            fill="url(#convictionGradient)"
            name="Conviction"
          />
          
          {/* Confidence line */}
          <Line
            type="monotone"
            dataKey="confidence"
            stroke="#8b5cf6"
            strokeWidth={2}
            dot={false}
            name="Confidence"
          />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
};

/**
 * Mini sparkline for quick overview
 */
export const ConvictionSparkline: React.FC<{
  data: number[];
  width?: number;
  height?: number;
}> = ({ data, width = 100, height = 30 }) => {
  const chartData = data.map((value, index) => ({ index, value: value * 100 }));
  
  return (
    <div style={{ width, height }}>
      <ResponsiveContainer>
        <LineChart data={chartData}>
          <Line
            type="monotone"
            dataKey="value"
            stroke={data[data.length - 1] > 0.6 ? '#10b981' : '#ef4444'}
            strokeWidth={2}
            dot={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
};

export default SignalStrengthChart;
