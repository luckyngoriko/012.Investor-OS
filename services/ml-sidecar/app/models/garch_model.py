"""GARCH volatility forecasting model (Sprint 120).

Uses the `arch` library for GARCH(1,1) fitting and volatility forecasting.
Computes VaR and CVaR at configurable confidence levels.
"""

import logging
import time

import numpy as np
from arch import arch_model
from scipy import stats

logger = logging.getLogger(__name__)

# Annualization factor (252 trading days, 365 for crypto)
TRADING_DAYS = 365  # crypto — change to 252 for equities


class GarchPredictor:
    """GARCH(1,1) volatility forecasting and risk metrics."""

    def __init__(self):
        self.model_version = "1.0.0"

    def predict(
        self,
        returns: list[float],
        horizon_days: int = 1,
        confidence_levels: list[float] | None = None,
    ) -> dict:
        """Fit GARCH(1,1) and forecast volatility + VaR/CVaR.

        Args:
            returns: Historical log returns (daily).
            horizon_days: Forecast horizon in days.
            confidence_levels: VaR confidence levels (default [0.95, 0.99]).

        Returns:
            Dict with volatility forecast, VaR, CVaR, and metadata.
        """
        if confidence_levels is None:
            confidence_levels = [0.95, 0.99]

        start = time.time()

        returns_arr = np.array(returns) * 100  # arch expects percentage returns

        if len(returns_arr) < 30:
            raise ValueError(f"Need at least 30 returns, got {len(returns_arr)}")

        # Fit GARCH(1,1) with Student-t distribution for fat tails
        model = arch_model(
            returns_arr,
            vol="Garch",
            p=1,
            q=1,
            dist="t",
            mean="Constant",
        )

        try:
            result = model.fit(disp="off", show_warning=False)
        except Exception as e:
            logger.warning("GARCH fit with t-dist failed, falling back to normal: %s", e)
            model = arch_model(returns_arr, vol="Garch", p=1, q=1, dist="normal")
            result = model.fit(disp="off", show_warning=False)

        # Forecast variance for horizon
        forecast = result.forecast(horizon=horizon_days)
        fv = forecast.variance
        # Handle both pandas DataFrame and numpy array API
        if hasattr(fv, 'iloc'):
            forecasted_variance = fv.iloc[-1].values
        else:
            forecasted_variance = np.asarray(fv)[-1]
        forecasted_vol_daily = np.sqrt(forecasted_variance[-1]) / 100  # back to decimal
        forecasted_vol_ann = forecasted_vol_daily * np.sqrt(TRADING_DAYS)

        # Current conditional volatility
        cond_vol = result.conditional_volatility
        last_cond_vol = float(cond_vol.iloc[-1]) if hasattr(cond_vol, 'iloc') else float(np.asarray(cond_vol)[-1])
        current_vol_daily = abs(last_cond_vol) / 100

        # VaR and CVaR computation
        var_results = {}
        cvar_results = {}

        for level in confidence_levels:
            z = stats.norm.ppf(1 - level)
            # Multi-day VaR: scale by sqrt(horizon)
            var_1d = z * forecasted_vol_daily
            var_horizon = var_1d * np.sqrt(horizon_days)
            var_results[f"var_{int(level * 100)}"] = round(float(var_horizon), 6)

            # CVaR (Expected Shortfall) — average loss beyond VaR
            cvar_1d = -stats.norm.pdf(z) / (1 - level) * forecasted_vol_daily
            cvar_horizon = cvar_1d * np.sqrt(horizon_days)
            cvar_results[f"cvar_{int(level * 100)}"] = round(float(cvar_horizon), 6)

        # Model parameters
        params = {
            "omega": round(float(result.params.get("omega", 0)), 8),
            "alpha": round(float(result.params.get("alpha[1]", 0)), 6),
            "beta": round(float(result.params.get("beta[1]", 0)), 6),
        }

        # Persistence (alpha + beta, should be < 1 for stationarity)
        persistence = params["alpha"] + params["beta"]

        # Confidence: lower if model fit is poor or persistence > 0.99
        confidence = 0.8
        if persistence > 0.99:
            confidence = 0.4  # near unit root — unreliable forecast
        elif persistence > 0.95:
            confidence = 0.6

        latency_ms = int((time.time() - start) * 1000)

        return {
            "forecasted_volatility_daily": round(float(forecasted_vol_daily), 6),
            "forecasted_volatility_ann": round(float(forecasted_vol_ann), 6),
            "current_volatility_daily": round(float(current_vol_daily), 6),
            "horizon_days": horizon_days,
            **var_results,
            **cvar_results,
            "persistence": round(float(persistence), 6),
            "garch_params": params,
            "confidence": round(confidence, 4),
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "n_observations": len(returns),
        }
