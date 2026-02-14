/**
 * Market Regime Indicator
 * Displays current market regime with appropriate styling
 */

import React from 'react';
import { MarketRegime } from '../types/hrm';

interface RegimeIndicatorProps {
  regime: MarketRegime;
  confidence?: number;
  showIcon?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

interface RegimeConfig {
  label: string;
  bgColor: string;
  textColor: string;
  borderColor: string;
  icon: string;
  description: string;
}

const regimeConfigs: Record<MarketRegime, RegimeConfig> = {
  Bull: {
    label: 'Bull Market',
    bgColor: 'bg-green-100',
    textColor: 'text-green-800',
    borderColor: 'border-green-300',
    icon: '🐂',
    description: 'Rising prices, optimistic sentiment',
  },
  Bear: {
    label: 'Bear Market',
    bgColor: 'bg-red-100',
    textColor: 'text-red-800',
    borderColor: 'border-red-300',
    icon: '🐻',
    description: 'Falling prices, pessimistic sentiment',
  },
  Sideways: {
    label: 'Sideways',
    bgColor: 'bg-gray-100',
    textColor: 'text-gray-800',
    borderColor: 'border-gray-300',
    icon: '↔️',
    description: 'Consolidation, range-bound',
  },
  Crisis: {
    label: 'Crisis',
    bgColor: 'bg-purple-100',
    textColor: 'text-purple-800',
    borderColor: 'border-purple-300',
    icon: '⚠️',
    description: 'High volatility, extreme fear',
  },
  StrongUptrend: {
    label: 'Strong Uptrend',
    bgColor: 'bg-emerald-100',
    textColor: 'text-emerald-800',
    borderColor: 'border-emerald-300',
    icon: '📈',
    description: 'Powerful upward momentum',
  },
  StrongDowntrend: {
    label: 'Strong Downtrend',
    bgColor: 'bg-rose-100',
    textColor: 'text-rose-800',
    borderColor: 'border-rose-300',
    icon: '📉',
    description: 'Powerful downward momentum',
  },
  Trending: {
    label: 'Trending',
    bgColor: 'bg-blue-100',
    textColor: 'text-blue-800',
    borderColor: 'border-blue-300',
    icon: '➡️',
    description: 'Clear directional movement',
  },
  Ranging: {
    label: 'Ranging',
    bgColor: 'bg-yellow-100',
    textColor: 'text-yellow-800',
    borderColor: 'border-yellow-300',
    icon: '⬌',
    description: 'Support and resistance levels',
  },
  Volatile: {
    label: 'Volatile',
    bgColor: 'bg-orange-100',
    textColor: 'text-orange-800',
    borderColor: 'border-orange-300',
    icon: '⚡',
    description: 'High volatility, uncertain direction',
  },
};

const sizeClasses = {
  sm: 'px-2 py-1 text-xs',
  md: 'px-4 py-2 text-sm',
  lg: 'px-6 py-3 text-base',
};

export const RegimeIndicator: React.FC<RegimeIndicatorProps> = ({
  regime,
  confidence,
  showIcon = true,
  size = 'md',
}) => {
  const config = regimeConfigs[regime] || regimeConfigs.Sideways;

  return (
    <div
      className={`
        inline-flex items-center gap-2 rounded-lg border-2
        ${config.bgColor}
        ${config.textColor}
        ${config.borderColor}
        ${sizeClasses[size]}
        transition-all duration-300
      `}
    >
      {showIcon && <span className="text-lg">{config.icon}</span>}
      <div className="flex flex-col">
        <span className="font-semibold">{config.label}</span>
        {confidence !== undefined && (
          <span className="text-xs opacity-75">
            Confidence: {(confidence * 100).toFixed(1)}%
          </span>
        )}
      </div>
    </div>
  );
};

/**
 * Detailed regime card with description
 */
export const RegimeCard: React.FC<{
  regime: MarketRegime;
  recommendedStrategy: string;
}> = ({ regime, recommendedStrategy }) => {
  const config = regimeConfigs[regime] || regimeConfigs.Sideways;

  return (
    <div className={`
      p-4 rounded-xl border-2
      ${config.bgColor}
      ${config.borderColor}
    `}>
      <div className="flex items-center gap-3 mb-2">
        <span className="text-3xl">{config.icon}</span>
        <div>
          <h3 className={`font-bold text-lg ${config.textColor}`}>
            {config.label}
          </h3>
          <p className="text-sm text-gray-600">{config.description}</p>
        </div>
      </div>
      <div className="mt-3 pt-3 border-t border-gray-200">
        <p className="text-sm">
          <span className="font-medium">Recommended Strategy:</span>{' '}
          <span className="font-bold text-blue-600">{recommendedStrategy}</span>
        </p>
      </div>
    </div>
  );
};

export default RegimeIndicator;
