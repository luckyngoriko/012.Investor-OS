/**
 * HRM Input Form
 * Form for entering market signals
 */

import React, { useState } from 'react';
import { HRMInferenceRequest } from '../types/hrm';

interface HRMInputFormProps {
  onSubmit: (data: HRMInferenceRequest) => void;
  loading?: boolean;
}

export const HRMInputForm: React.FC<HRMInputFormProps> = ({
  onSubmit,
  loading = false,
}) => {
  const [formData, setFormData] = useState<HRMInferenceRequest>({
    pegy: 0.5,
    insider: 0.5,
    sentiment: 0.5,
    vix: 20,
    regime: 0,
    time: 0.5,
  });

  const handleChange = (field: keyof HRMInferenceRequest, value: number) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(formData);
  };

  const presets = {
    'Strong Bull': { pegy: 0.9, insider: 0.9, sentiment: 0.9, vix: 10, regime: 0 },
    'Moderate Bull': { pegy: 0.7, insider: 0.7, sentiment: 0.7, vix: 15, regime: 0 },
    'Bear Market': { pegy: 0.2, insider: 0.2, sentiment: 0.2, vix: 50, regime: 1 },
    'Crisis': { pegy: 0.1, insider: 0.1, sentiment: 0.1, vix: 80, regime: 3 },
    'Sideways': { pegy: 0.5, insider: 0.5, sentiment: 0.5, vix: 25, regime: 2 },
  };

  const applyPreset = (preset: keyof typeof presets) => {
    setFormData((prev) => ({ ...prev, ...presets[preset] }));
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      {/* Presets */}
      <div className="flex flex-wrap gap-2 mb-4">
        <span className="text-sm text-gray-500 self-center">Presets:</span>
        {Object.keys(presets).map((preset) => (
          <button
            key={preset}
            type="button"
            onClick={() => applyPreset(preset as keyof typeof presets)}
            className="px-3 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded-full transition-colors"
          >
            {preset}
          </button>
        ))}
      </div>

      {/* PEGY Score */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          PEGY Score (Valuation)
        </label>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={formData.pegy}
          onChange={(e) => handleChange('pegy', parseFloat(e.target.value))}
          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-xs text-gray-500">
          <span>Expensive (0)</span>
          <span className="font-medium text-blue-600">
            {(formData.pegy * 100).toFixed(0)}%
          </span>
          <span>Cheap (1)</span>
        </div>
      </div>

      {/* Insider Activity */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          Insider Activity
        </label>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={formData.insider}
          onChange={(e) => handleChange('insider', parseFloat(e.target.value))}
          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-xs text-gray-500">
          <span>Selling (0)</span>
          <span className="font-medium text-blue-600">
            {(formData.insider * 100).toFixed(0)}%
          </span>
          <span>Buying (1)</span>
        </div>
      </div>

      {/* Market Sentiment */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          Market Sentiment
        </label>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={formData.sentiment}
          onChange={(e) => handleChange('sentiment', parseFloat(e.target.value))}
          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-xs text-gray-500">
          <span>Fear (0)</span>
          <span className="font-medium text-blue-600">
            {(formData.sentiment * 100).toFixed(0)}%
          </span>
          <span>Greed (1)</span>
        </div>
      </div>

      {/* VIX */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          VIX (Volatility Index)
        </label>
        <input
          type="range"
          min="10"
          max="80"
          step="1"
          value={formData.vix}
          onChange={(e) => handleChange('vix', parseFloat(e.target.value))}
          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-xs text-gray-500">
          <span>Low (10)</span>
          <span className="font-medium text-blue-600">
            {formData.vix.toFixed(0)}
          </span>
          <span>High (80)</span>
        </div>
      </div>

      {/* Market Regime */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          Market Regime
        </label>
        <select
          value={formData.regime}
          onChange={(e) => handleChange('regime', parseInt(e.target.value))}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value={0}>🐂 Bull Market</option>
          <option value={1}>🐻 Bear Market</option>
          <option value={2}>↔️ Sideways</option>
          <option value={3}>⚠️ Crisis</option>
        </select>
      </div>

      {/* Submit Button */}
      <button
        type="submit"
        disabled={loading}
        className={`
          w-full py-3 px-4 rounded-lg font-medium text-white
          transition-all duration-200
          ${loading
            ? 'bg-gray-400 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700 active:bg-blue-800'
          }
        `}
      >
        {loading ? (
          <span className="flex items-center justify-center gap-2">
            <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24">
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
                fill="none"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
              />
            </svg>
            Processing...
          </span>
        ) : (
          'Run HRM Analysis'
        )}
      </button>
    </form>
  );
};

export default HRMInputForm;
