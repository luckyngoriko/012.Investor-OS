# Plan: Integrate sapientinc/HRM as Python NATS Worker

## Date: 2026-04-02

## Status: PENDING

## Why

Our current "HRM" is a 9K-param feedforward net pretending to be HRM. The real sapientinc/HRM is a 27M-param Transformer with Adaptive Computation Time — designed for fast learning with minimal data. Apache 2.0 license.

## Steps

### 1. Clone and adapt sapientinc/HRM

- Clone https://github.com/sapientinc/HRM into services/ml-gpu/hrm/
- Use HRM-mini config (smaller, fits RTX 3090 easily)
- Create trading adapter: OHLCV → tokenized sequence → HRM → action tokens
- Reuse SMC indicators concept from Daydreamer20 (reimplemented, not copied)

### 2. Training adapter

- Input: our 1440 OHLCV candles from prices table
- Feature engineering: RSI, MACD, BB, ATR, volume z-score + price action tokens
- Target: next-period action (buy/sell/hold based on actual future return)
- Train on RTX 3090 — should be fast (HRM's key advantage)

### 3. NATS worker

- services/ml-gpu/app/workers/hrm_worker.py
- Subscribes to ios.features.{symbol} + ios.sentiment.{symbol}
- Runs HRM inference on GPU
- Publishes to ios.predict.hrm.{symbol}
- Replaces the Rust HRM worker (which becomes fallback)

### 4. Config

- HRM-mini: hidden=256, heads=8, H_layers=3, L_layers=3, H_cycles=2, L_cycles=2
- ~2-3M params, <500MB VRAM
- Training: 50 epochs on real data, ~30 min on RTX 3090

### 5. Rust HRM → fallback only

- Keep src/nats/hrm_worker.rs as fallback when GPU is unavailable
- Update consensus to weight real HRM higher than fallback

## Dependencies

- sapientinc/HRM (Apache 2.0)
- flash-attn (for GPU) or F.scaled_dot_product_attention fallback
- torch, einops

## Estimated effort: 1 day
