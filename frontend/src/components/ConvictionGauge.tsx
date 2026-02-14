/**
 * Conviction Gauge Component
 * Circular progress indicator for HRM conviction
 */

import React from 'react';

interface ConvictionGaugeProps {
  value: number; // 0.0 - 1.0
  size?: number; // pixels
  strokeWidth?: number;
}

export const ConvictionGauge: React.FC<ConvictionGaugeProps> = ({
  value,
  size = 200,
  strokeWidth = 20,
}) => {
  const percentage = Math.min(Math.max(value * 100, 0), 100);
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const offset = circumference - (percentage / 100) * circumference;

  // Color based on conviction level
  const getColor = (pct: number): string => {
    if (pct >= 80) return '#10b981'; // green-500 - Strong buy
    if (pct >= 60) return '#3b82f6'; // blue-500 - Buy
    if (pct >= 40) return '#f59e0b'; // amber-500 - Neutral
    if (pct >= 20) return '#f97316'; // orange-500 - Weak
    return '#ef4444'; // red-500 - Avoid
  };

  const color = getColor(percentage);

  return (
    <div className="flex flex-col items-center justify-center">
      <div className="relative" style={{ width: size, height: size }}>
        {/* Background circle */}
        <svg
          width={size}
          height={size}
          className="transform -rotate-90"
        >
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke="#e5e7eb"
            strokeWidth={strokeWidth}
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeDasharray={circumference}
            strokeDashoffset={offset}
            strokeLinecap="round"
            className="transition-all duration-500 ease-out"
          />
        </svg>

        {/* Center text */}
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-4xl font-bold" style={{ color }}>
            {percentage.toFixed(1)}%
          </span>
          <span className="text-sm text-gray-500 mt-1">Conviction</span>
        </div>
      </div>

      {/* Legend */}
      <div className="flex gap-2 mt-4 text-xs">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-red-500"></div>
          <span>Avoid</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-amber-500"></div>
          <span>Neutral</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-blue-500"></div>
          <span>Buy</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-green-500"></div>
          <span>Strong</span>
        </div>
      </div>
    </div>
  );
};

export default ConvictionGauge;
