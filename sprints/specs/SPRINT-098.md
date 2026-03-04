# Sprint 098: Data Integrity & Risk Analytics Production Wiring

## Metadata

- Sprint ID: 98
- Status: queued
- Gate: G45
- Owner: Backend + ML
- Dependencies: 96

## Objective

Replace synthetic data sources in collectors and placeholder analytics calculations with real implementations so that trading signals, risk metrics, and search results reflect actual market conditions.

## Scope In

1. News collector: replace synthetic articles with real RSS/API feed parsing.
2. Social collector: replace hardcoded tickers and posts with real Reddit/Twitter API or configurable feed integration.
3. VaR calculation: implement actual Monte Carlo simulation from portfolio positions.
4. RAG search similarity: compute real cosine similarity from embeddings instead of hardcoded 0.5.
5. Risk analytics: calculate real beta, alpha, information_ratio from benchmark returns.
6. Analytics handler: replace `generate_simulated_returns()` with real portfolio return query from database.
7. Backtest: calculate real `avg_trade_return` from trade history.
8. Claude API: parse `risk_rating` from LLM response instead of hardcoding "medium".

## Scope Out

1. New collector types (options, alternative data).
2. New ML models or training pipelines.
3. Frontend changes.

## Work Packages

1. WP-98A: News collector — implement RSS feed parsing using `reqwest` + XML/Atom parser. Support configurable feed URLs. Fallback: return empty vec with warning log if feeds unreachable.
2. WP-98B: Social collector — implement Reddit API integration (public JSON endpoints, no OAuth required for read). `trending_tickers()` queries actual subreddit data. Fallback: empty vec with log.
3. WP-98C: VaR Monte Carlo — implement parametric VaR using historical returns from position data. Use actual portfolio weights and correlations. Minimum 10,000 simulations.
4. WP-98D: RAG cosine similarity — compute `dot_product(a, b) / (norm(a) * norm(b))` between query embedding and stored document embeddings.
5. WP-98E: Risk metrics — fetch benchmark returns (configurable ticker, default SPY), calculate beta via covariance/variance, alpha via Jensen's alpha, information_ratio from tracking error.
6. WP-98F: Analytics returns — query `portfolio_snapshots` or `trade_history` table for actual daily returns. If no data, return error instead of simulated data.
7. WP-98G: Backtest avg_trade_return — calculate from completed trade history: `sum(trade_pnl) / count(trades)`.
8. WP-98H: Claude risk_rating — parse LLM response for risk keywords (high/medium/low) or structured JSON output.

## Acceptance Criteria

1. News collector returns real articles when feeds are reachable; empty vec with warning when not.
2. Social collector queries real data sources; no hardcoded ticker lists.
3. VaR result varies based on actual portfolio composition.
4. RAG search results are ranked by computed similarity scores.
5. Beta and alpha calculated against real benchmark data.
6. Analytics endpoints return real portfolio data or explicit "no data" error.
7. Backtest metrics reflect actual trade calculations.
8. 10-K risk ratings parsed from LLM analysis content.
9. All existing tests pass; new tests verify computed output.

## Verification Commands

```bash
cargo clippy -- -D warnings
cargo test --lib -- --test-threads=4
cargo test --test '*' -- --test-threads=2
```

## Files to Modify

- `src/collectors/news/mod.rs` (WP-98A)
- `src/collectors/social/mod.rs` (WP-98B)
- `src/risk/advanced/mod.rs` (WP-98C)
- `src/rag/search/mod.rs` (WP-98D)
- `src/analytics/risk/mod.rs` (WP-98E)
- `src/api/handlers/analytics.rs` (WP-98F)
- `src/analytics/backtest/mod.rs` (WP-98G)
- `src/ml/apis/claude.rs` (WP-98H)
