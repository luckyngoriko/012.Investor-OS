#!/usr/bin/env python3
"""Fetch crypto news and analyze sentiment with FinBERT (Phase C5).

Uses CoinGecko's free news API (no key required) to get latest crypto headlines,
then feeds them to the FinBERT model running in the ML sidecar.

Usage:
    python scripts/ml/fetch_news_sentiment.py
    python scripts/ml/fetch_news_sentiment.py --symbol BTCUSDT --sidecar http://localhost:9000
"""

import argparse
import json
import logging
import os
import sys

import psycopg2
import requests

logger = logging.getLogger(__name__)

# Free crypto news sources (no API key)
COINGECKO_NEWS = "https://api.coingecko.com/api/v3/news"

DB_URL = os.environ.get("DATABASE_URL", "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os")

# Symbol to search terms mapping
SYMBOL_KEYWORDS = {
    "BTCUSDT": ["bitcoin", "btc", "Bitcoin"],
    "ETHUSDT": ["ethereum", "eth", "Ethereum"],
}


def fetch_crypto_news(max_articles: int = 10) -> list[dict]:
    """Fetch latest crypto news from CoinGecko (free, no API key)."""
    try:
        resp = requests.get(COINGECKO_NEWS, timeout=10)
        resp.raise_for_status()
        data = resp.json()

        articles = []
        for item in data.get("data", [])[:max_articles]:
            articles.append({
                "title": item.get("title", ""),
                "description": item.get("description", "")[:200],
                "source": item.get("author", "unknown"),
                "url": item.get("url", ""),
            })

        return articles
    except Exception as e:
        logger.error("Failed to fetch news: %s", e)
        return []


def classify_article_symbol(title: str, description: str) -> list[str]:
    """Determine which symbols an article is relevant to."""
    text = (title + " " + description).lower()
    symbols = []
    for symbol, keywords in SYMBOL_KEYWORDS.items():
        if any(kw.lower() in text for kw in keywords):
            symbols.append(symbol)
    return symbols if symbols else ["BTCUSDT"]  # Default to BTC for general crypto news


def analyze_sentiment_via_sidecar(texts: list[str], sidecar_url: str) -> list[dict] | None:
    """Send texts to FinBERT endpoint in the ML sidecar."""
    try:
        resp = requests.post(
            f"{sidecar_url}/v1/predict/finbert",
            json={"texts": texts[:16]},
            timeout=30,
        )
        if resp.status_code == 503:
            logger.warning("FinBERT not loaded on sidecar")
            return None
        resp.raise_for_status()
        data = resp.json()
        return data.get("results", [])
    except Exception as e:
        logger.error("FinBERT request failed: %s", e)
        return None


def store_sentiment(cur, symbol: str, sentiment_data: dict):
    """Store sentiment features in ml_feature_store."""
    cur.execute(
        "INSERT INTO ml_feature_store (id, symbol, feature_set, features, source, computed_at) "
        "VALUES (gen_random_uuid(), %s, 'sentiment', %s, 'finbert_news', NOW())",
        (symbol, json.dumps(sentiment_data)),
    )


def run_news_sentiment(sidecar_url: str = "http://ios-ml-sidecar:9000"):
    """Full pipeline: fetch news → classify → sentiment → store."""
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")

    # 1. Fetch news
    articles = fetch_crypto_news(max_articles=10)
    if not articles:
        print("No news articles found")
        return

    print(f"Fetched {len(articles)} news articles")

    # 2. Analyze sentiment
    titles = [a["title"] for a in articles]
    sentiments = analyze_sentiment_via_sidecar(titles, sidecar_url)

    if sentiments is None:
        # Fallback: simple keyword-based sentiment
        print("FinBERT unavailable, using keyword fallback")
        positive_words = {"surge", "bull", "rally", "gain", "rise", "high", "up", "growth", "profit", "approve"}
        negative_words = {"crash", "bear", "fall", "drop", "low", "down", "loss", "hack", "ban", "delay"}

        sentiments = []
        for title in titles:
            words = set(title.lower().split())
            pos = len(words & positive_words)
            neg = len(words & negative_words)
            score = (pos - neg) / max(pos + neg, 1)
            sentiments.append({
                "text": title[:100],
                "sentiment": "positive" if score > 0 else "negative" if score < 0 else "neutral",
                "score": round(score, 4),
                "confidence": 0.5,
            })

    # 3. Aggregate per symbol
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()

    symbol_sentiments: dict[str, list] = {}
    for article, sent in zip(articles, sentiments):
        symbols = classify_article_symbol(article["title"], article.get("description", ""))
        for symbol in symbols:
            if symbol not in symbol_sentiments:
                symbol_sentiments[symbol] = []
            symbol_sentiments[symbol].append(sent)

    for symbol, sents in symbol_sentiments.items():
        scores = [s.get("score", 0) for s in sents]
        avg_score = sum(scores) / len(scores) if scores else 0
        pos_count = sum(1 for s in sents if s.get("sentiment") == "positive")
        neg_count = sum(1 for s in sents if s.get("sentiment") == "negative")
        neu_count = sum(1 for s in sents if s.get("sentiment") == "neutral")

        sentiment_data = {
            "avg_sentiment_score": round(avg_score, 4),
            "positive_count": pos_count,
            "negative_count": neg_count,
            "neutral_count": neu_count,
            "total_articles": len(sents),
            "headlines": [s.get("text", "")[:80] for s in sents[:5]],
        }

        store_sentiment(cur, symbol, sentiment_data)
        print(f"  {symbol}: avg={avg_score:.3f} pos={pos_count} neg={neg_count} neu={neu_count} ({len(sents)} articles)")

    conn.commit()
    cur.close()
    conn.close()
    print("Sentiment stored in ml_feature_store")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sidecar", default="http://ios-ml-sidecar:9000")
    args = parser.parse_args()
    run_news_sentiment(args.sidecar)
