# Sprint 098 Report: Data Integrity & Risk Analytics Production Wiring

## Sprint Result

- Status: done
- Gate: PASS (G45)
- Scope completion: 100%
- Started: 2026-03-04
- Completed: 2026-03-04

## Delivered

1. **WP-98A: News collector RSS feeds** — Replaced synthetic article generation with real HTTP RSS fetching via `reqwest`. Added `parse_rss_xml()`, `extract_tag()`, `extract_attr()` for XML/Atom parsing. Filters articles by ticker mention in title. Graceful fallback: empty vec with warning log when feeds unreachable.
2. **WP-98B: Social collector real APIs** — Reddit: replaced hardcoded posts with real Reddit JSON API calls to r/wallstreetbets, r/stocks, r/investing. `trending_tickers()` extracts $TICKER patterns from live posts via regex. Twitter: real API v2 integration with Bearer token auth; returns empty vec gracefully when `TWITTER_BEARER_TOKEN` not set.
3. **WP-98C: VaR Monte Carlo** — Replaced placeholder (`Decimal::from(1000)`, `0.05`) with real Monte Carlo simulation. Uses actual portfolio position weights, Box-Muller normal RNG, 100K simulations, sqrt-t scaling for time horizon. VaR varies based on portfolio composition.
4. **WP-98D: RAG cosine similarity** — Replaced hardcoded `similarity = 0.5` in `search_journal()` with pgvector `<=>` (cosine distance) operator. Results ranked by `1.0 - distance`. Added `JournalDecisionRow` struct with `FromRow` impl.
5. **WP-98E: Risk beta/alpha/information_ratio** — Added `benchmark_returns` field to `RiskAnalyzer` with `with_benchmark()` builder. `calculate_all()` now computes: beta via cov(port,bench)/var(bench), Jensen's alpha, information ratio from tracking error. Returns `None` when no benchmark provided (backwards compatible).
6. **WP-98F: Analytics real returns** — Replaced `generate_simulated_returns()` (random data) with `fetch_portfolio_returns()` that queries `portfolio_snapshots` table via sqlx. Returns explicit error when insufficient data instead of fake numbers.
7. **WP-98G: Backtest avg_trade_return** — Fixed `calculate_result()`: replaced `filter(|_| false)` (always 0 wins) with FIFO buy/sell pair matching. Computes real `avg_trade_return` from completed round-trip trades and P&L. Winning/losing trade counts are now accurate.
8. **WP-98H: Claude risk_rating** — Replaced hardcoded `"medium"` with keyword parsing from LLM analysis text. Scans for "high/significant/elevated/substantial risk" → high, "low/minimal/limited/negligible risk" → low, else medium.

## Files Modified

- `src/collectors/news/mod.rs` (WP-98A)
- `src/collectors/social/mod.rs` (WP-98B)
- `src/risk/advanced/mod.rs` (WP-98C)
- `src/rag/search/mod.rs` (WP-98D)
- `src/analytics/risk/mod.rs` (WP-98E)
- `src/api/handlers/analytics.rs` (WP-98F)
- `src/analytics/backtest/mod.rs` (WP-98G)
- `src/ml/apis/claude.rs` (WP-98H)

## Verification Evidence

```bash
cargo clippy -- -D warnings   # PASS: 0 warnings
cargo test --lib -- --test-threads=4  # PASS: 261 passed, 0 failed, 2 ignored
cargo check                   # PASS
```
