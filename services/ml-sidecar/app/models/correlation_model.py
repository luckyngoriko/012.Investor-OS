"""Cross-asset dynamic correlation model using DCC-GARCH (Wave 2 Task 10).

Simplified DCC-GARCH approach:
1. Fit individual GARCH(1,1) per asset to get standardized residuals
2. Compute dynamic correlation from exponentially weighted correlation of residuals
3. Return time-varying correlation matrix and most/least correlated pairs
"""

import logging
import time
from itertools import combinations

import numpy as np
from arch import arch_model

logger = logging.getLogger(__name__)

# Default assets for cross-correlation analysis
DEFAULT_ASSETS = ["BTC", "ETH", "SPY", "GOLD", "DXY"]

# Exponential weighting decay factor for dynamic correlation
DEFAULT_DECAY = 0.94


class CorrelationPredictor:
    """Dynamic cross-asset correlation via simplified DCC-GARCH."""

    def __init__(self):
        self.model_version = "1.0.0"

    def predict(
        self,
        returns: dict[str, list[float]],
        decay: float = DEFAULT_DECAY,
        top_n: int = 5,
    ) -> dict:
        """Compute dynamic cross-asset correlation matrix.

        Args:
            returns: Dict mapping symbol -> list of daily log returns.
                     All series must have the same length.
            decay: Exponential decay factor for EWMA correlation (0 < decay < 1).
            top_n: Number of top correlated / uncorrelated pairs to return.

        Returns:
            Dict with correlation_matrix, top_correlated, top_uncorrelated,
            per-asset GARCH diagnostics, and metadata.
        """
        start = time.time()

        symbols = sorted(returns.keys())
        n_assets = len(symbols)

        if n_assets < 2:
            raise ValueError(f"Need at least 2 assets, got {n_assets}")

        # Validate all series have equal length
        lengths = {sym: len(r) for sym, r in returns.items()}
        unique_lengths = set(lengths.values())
        if len(unique_lengths) > 1:
            raise ValueError(f"All return series must have equal length, got {lengths}")

        n_obs = lengths[symbols[0]]
        if n_obs < 30:
            raise ValueError(f"Need at least 30 observations, got {n_obs}")

        # Step 1: Fit individual GARCH(1,1) and extract standardized residuals
        std_residuals = {}
        garch_diagnostics = {}

        for sym in symbols:
            ret_arr = np.array(returns[sym]) * 100  # arch expects percentage returns

            model = arch_model(
                ret_arr,
                vol="Garch",
                p=1,
                q=1,
                dist="t",
                mean="Constant",
            )

            try:
                result = model.fit(disp="off", show_warning=False)
            except Exception:
                # Fallback to normal distribution
                model = arch_model(ret_arr, vol="Garch", p=1, q=1, dist="normal")
                result = model.fit(disp="off", show_warning=False)

            # Standardized residuals = residuals / conditional_volatility
            resid = np.asarray(result.resid)
            cond_vol = np.asarray(result.conditional_volatility)

            # Avoid division by zero
            cond_vol_safe = np.where(cond_vol > 1e-10, cond_vol, 1e-10)
            z = resid / cond_vol_safe
            std_residuals[sym] = z

            alpha = float(result.params.get("alpha[1]", 0))
            beta = float(result.params.get("beta[1]", 0))
            persistence = alpha + beta

            garch_diagnostics[sym] = {
                "alpha": round(alpha, 6),
                "beta": round(beta, 6),
                "persistence": round(persistence, 6),
                "current_vol_daily": round(
                    float(abs(cond_vol[-1])) / 100, 6
                ),
            }

        # Step 2: Build matrix of standardized residuals [T x N]
        Z = np.column_stack([std_residuals[sym] for sym in symbols])

        # Step 3: Compute EWMA dynamic correlation
        corr_matrix = self._ewma_correlation(Z, decay)

        # Step 4: Build output correlation matrix as dict
        corr_dict = {}
        for i, sym_i in enumerate(symbols):
            row = {}
            for j, sym_j in enumerate(symbols):
                row[sym_j] = round(float(corr_matrix[i, j]), 6)
            corr_dict[sym_i] = row

        # Step 5: Identify most/least correlated pairs
        pairs = []
        for i, j in combinations(range(n_assets), 2):
            corr_val = float(corr_matrix[i, j])
            pairs.append({
                "pair": f"{symbols[i]}/{symbols[j]}",
                "asset_a": symbols[i],
                "asset_b": symbols[j],
                "correlation": round(corr_val, 6),
                "abs_correlation": round(abs(corr_val), 6),
            })

        # Sort by absolute correlation descending for top correlated
        pairs_sorted = sorted(pairs, key=lambda p: p["abs_correlation"], reverse=True)
        top_correlated = pairs_sorted[:top_n]

        # Least correlated = lowest absolute correlation
        top_uncorrelated = sorted(pairs, key=lambda p: p["abs_correlation"])[:top_n]

        # Confidence: average quality of individual GARCH fits
        avg_persistence = np.mean(
            [garch_diagnostics[s]["persistence"] for s in symbols]
        )
        confidence = 0.8
        if avg_persistence > 0.99:
            confidence = 0.4
        elif avg_persistence > 0.95:
            confidence = 0.6

        latency_ms = int((time.time() - start) * 1000)

        return {
            "symbols": symbols,
            "n_assets": n_assets,
            "n_observations": n_obs,
            "decay_factor": decay,
            "correlation_matrix": corr_dict,
            "top_correlated": [
                {k: v for k, v in p.items() if k != "abs_correlation"}
                for p in top_correlated
            ],
            "top_uncorrelated": [
                {k: v for k, v in p.items() if k != "abs_correlation"}
                for p in top_uncorrelated
            ],
            "garch_diagnostics": garch_diagnostics,
            "confidence": round(confidence, 4),
            "latency_ms": latency_ms,
            "model_version": self.model_version,
        }

    @staticmethod
    def _ewma_correlation(Z: np.ndarray, decay: float) -> np.ndarray:
        """Compute exponentially weighted moving average correlation matrix.

        Uses the final EWMA covariance estimate to produce a dynamic
        correlation snapshot that reflects recent co-movement patterns
        more strongly than distant ones.

        Args:
            Z: Standardized residuals matrix [T x N].
            decay: Decay factor (lambda). Higher = more weight on older data.

        Returns:
            N x N correlation matrix.
        """
        T, N = Z.shape

        # Initialize with sample covariance of first window
        init_window = min(30, T)
        S = np.cov(Z[:init_window].T)

        # EWMA update: S_t = decay * S_{t-1} + (1 - decay) * z_t * z_t'
        for t in range(init_window, T):
            z_t = Z[t].reshape(-1, 1)
            S = decay * S + (1 - decay) * (z_t @ z_t.T)

        # Convert covariance to correlation
        diag = np.sqrt(np.diag(S))
        diag_safe = np.where(diag > 1e-10, diag, 1e-10)
        D_inv = np.diag(1.0 / diag_safe)
        corr = D_inv @ S @ D_inv

        # Ensure diagonal is exactly 1 and values are clipped
        np.fill_diagonal(corr, 1.0)
        corr = np.clip(corr, -1.0, 1.0)

        return corr
