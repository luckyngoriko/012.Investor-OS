//! Template-based response generator for the AI Chat module.
//!
//! Analyzes the user question and the retrieved `ChatContext` to produce
//! a structured Markdown response. No external LLM calls — pure
//! template logic for now.

use super::context::ChatContext;
use serde_json::Value;

/// Generate a template-based response from the context and user question.
///
/// Detects the question intent and routes to the appropriate template:
/// - "why buy/sell" → consensus + model contributions + features
/// - "risk" → GARCH volatility + VaR information
/// - "performing / accuracy" → prediction accuracy from ml_predictions
/// - fallback → general summary
pub fn generate_response(context: &ChatContext, question: &str) -> (String, Vec<String>) {
    let q_lower = question.to_lowercase();
    let sources = context.sources.clone();

    let answer = if q_lower.contains("why")
        && (q_lower.contains("buy") || q_lower.contains("sell") || q_lower.contains("trade"))
    {
        generate_why_trade(context)
    } else if q_lower.contains("risk")
        || q_lower.contains("volatility")
        || q_lower.contains("var")
        || q_lower.contains("drawdown")
    {
        generate_risk_answer(context)
    } else if q_lower.contains("perform")
        || q_lower.contains("accuracy")
        || q_lower.contains("how is")
    {
        generate_performance_answer(context)
    } else {
        generate_general_summary(context, question)
    };

    (answer, sources)
}

/// Template: Why did AI buy/sell X?
fn generate_why_trade(ctx: &ChatContext) -> String {
    let mut parts = Vec::new();

    parts.push(format!("## AI Trading Analysis for {}\n", ctx.symbol));

    // Consensus from predictions
    if ctx.predictions.is_empty() {
        parts.push(
            "**No recent predictions found** for this symbol in the last 24 hours.\n".to_string(),
        );
    } else {
        parts.push("### Model Consensus\n".to_string());

        let avg_confidence = average_confidence(&ctx.predictions);
        parts.push(format!(
            "- **Models contributing:** {}\n- **Average confidence:** {:.1}%\n",
            ctx.predictions.len(),
            avg_confidence * 100.0
        ));

        // List each model's contribution
        parts.push("\n### Model Contributions\n".to_string());
        for pred in &ctx.predictions {
            let model = pred["model_name"].as_str().unwrap_or("unknown");
            let conf = pred["confidence"].as_f64().unwrap_or(0.0);
            let pred_type = pred["prediction_type"].as_str().unwrap_or("unknown");
            let horizon = pred["horizon"].as_str().unwrap_or("N/A");
            let predicted = &pred["predicted_value"];

            parts.push(format!(
                "| **{model}** | type: {pred_type} | horizon: {horizon} | confidence: {:.1}% | value: {} |\n",
                conf * 100.0,
                format_predicted_value(predicted)
            ));
        }
    }

    // AI decisions
    if !ctx.decisions.is_empty() {
        parts.push("\n### AI Decision Reasoning\n".to_string());
        for decision in &ctx.decisions {
            let dtype = decision["decision_type"].as_str().unwrap_or("unknown");
            let explanation = decision["explanation"].as_str().unwrap_or("No explanation");
            let conf = decision["confidence"].as_f64().unwrap_or(0.0);
            parts.push(format!(
                "- **{dtype}** (confidence: {:.1}%): {explanation}\n",
                conf * 100.0
            ));
        }
    }

    // Features snapshot
    append_features_summary(&mut parts, ctx);

    parts.join("")
}

/// Template: What is the risk?
fn generate_risk_answer(ctx: &ChatContext) -> String {
    let mut parts = Vec::new();

    parts.push(format!("## Risk Analysis for {}\n", ctx.symbol));

    // Look for volatility predictions
    let vol_predictions: Vec<&Value> = ctx
        .predictions
        .iter()
        .filter(|p| {
            p["prediction_type"].as_str() == Some("volatility")
                || p["model_name"]
                    .as_str()
                    .map(|n| n.to_lowercase().contains("garch"))
                    .unwrap_or(false)
        })
        .collect();

    if vol_predictions.is_empty() {
        parts.push("**No GARCH/volatility predictions** found in the last 24 hours.\n".to_string());
    } else {
        parts.push("### Volatility Forecast (GARCH)\n".to_string());
        for vp in &vol_predictions {
            let model = vp["model_name"].as_str().unwrap_or("GARCH");
            let horizon = vp["horizon"].as_str().unwrap_or("N/A");
            let predicted = &vp["predicted_value"];
            let conf = vp["confidence"].as_f64().unwrap_or(0.0);
            parts.push(format!(
                "- **{model}** (horizon: {horizon}, confidence: {:.1}%): {}\n",
                conf * 100.0,
                format_predicted_value(predicted)
            ));
        }
    }

    // Extract VaR from features if available
    let var_info = extract_feature_value(ctx, "var");
    if let Some(var_val) = var_info {
        parts.push(format!("\n### Value at Risk (VaR)\n- {var_val}\n"));
    }

    // Extract drawdown from features
    let dd_info = extract_feature_value(ctx, "drawdown");
    if let Some(dd_val) = dd_info {
        parts.push(format!("\n### Maximum Drawdown\n- {dd_val}\n"));
    }

    // Features snapshot
    append_features_summary(&mut parts, ctx);

    if parts.len() == 1 {
        parts.push(
            "No risk-specific data available. Check back after models have run.\n".to_string(),
        );
    }

    parts.join("")
}

/// Template: How is X performing?
fn generate_performance_answer(ctx: &ChatContext) -> String {
    let mut parts = Vec::new();

    parts.push(format!("## Prediction Performance for {}\n", ctx.symbol));

    if ctx.predictions.is_empty() {
        parts.push("**No predictions found** for this symbol.\n".to_string());
        return parts.join("");
    }

    // Overall stats
    let total = ctx.predictions.len();
    let resolved: Vec<&Value> = ctx
        .predictions
        .iter()
        .filter(|p| !p["actual_value"].is_null())
        .collect();
    let fallbacks = ctx
        .predictions
        .iter()
        .filter(|p| p["is_fallback"].as_bool() == Some(true))
        .count();

    let avg_confidence = average_confidence(&ctx.predictions);

    parts.push("### Summary\n".to_string());
    parts.push(format!("- **Total predictions (24h):** {total}\n"));
    parts.push(format!("- **Resolved:** {}\n", resolved.len()));
    parts.push(format!("- **Unresolved:** {}\n", total - resolved.len()));
    parts.push(format!("- **Fallback predictions:** {fallbacks}\n"));
    parts.push(format!(
        "- **Average confidence:** {:.1}%\n",
        avg_confidence * 100.0
    ));

    // Error metrics for resolved predictions
    if !resolved.is_empty() {
        let errors: Vec<f64> = resolved
            .iter()
            .filter_map(|p| p["error_metric"].as_f64())
            .collect();
        if !errors.is_empty() {
            let avg_error: f64 = errors.iter().sum::<f64>() / errors.len() as f64;
            parts.push(format!(
                "\n### Accuracy Metrics\n- **Average error (MAE/RMSE):** {:.4}\n- **Evaluated predictions:** {}\n",
                avg_error,
                errors.len()
            ));
        }
    }

    // Per-model breakdown
    let mut models: std::collections::HashMap<String, (usize, f64)> =
        std::collections::HashMap::new();
    for pred in &ctx.predictions {
        let name = pred["model_name"].as_str().unwrap_or("unknown").to_string();
        let conf = pred["confidence"].as_f64().unwrap_or(0.0);
        let entry = models.entry(name).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += conf;
    }
    if !models.is_empty() {
        parts.push("\n### Per-Model Breakdown\n".to_string());
        for (model, (count, total_conf)) in &models {
            parts.push(format!(
                "- **{model}:** {count} predictions, avg confidence: {:.1}%\n",
                (total_conf / *count as f64) * 100.0
            ));
        }
    }

    parts.join("")
}

/// Fallback: general summary of available data.
fn generate_general_summary(ctx: &ChatContext, question: &str) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "## {} — General Summary\n\nYou asked: *\"{}\"*\n\n",
        ctx.symbol, question
    ));

    parts.push(format!(
        "### Available Data\n- **Predictions (24h):** {}\n- **AI decisions (24h):** {}\n- **Feature snapshots:** {}\n",
        ctx.predictions.len(),
        ctx.decisions.len(),
        ctx.features.len()
    ));

    if !ctx.predictions.is_empty() {
        let avg_conf = average_confidence(&ctx.predictions);
        parts.push(format!(
            "\n### Latest Prediction\n- **Model:** {}\n- **Type:** {}\n- **Confidence:** {:.1}%\n",
            ctx.predictions[0]["model_name"]
                .as_str()
                .unwrap_or("unknown"),
            ctx.predictions[0]["prediction_type"]
                .as_str()
                .unwrap_or("unknown"),
            avg_conf * 100.0
        ));
    }

    append_features_summary(&mut parts, ctx);

    parts.push(
        "\n---\n*Try asking: \"Why buy/sell?\", \"What is the risk?\", or \"How is it performing?\"*\n"
            .to_string(),
    );

    parts.join("")
}

// ───────────────────────── helpers ─────────────────────────

/// Calculate average confidence across predictions.
fn average_confidence(predictions: &[Value]) -> f64 {
    if predictions.is_empty() {
        return 0.0;
    }
    let sum: f64 = predictions
        .iter()
        .map(|p| p["confidence"].as_f64().unwrap_or(0.0))
        .sum();
    sum / predictions.len() as f64
}

/// Format a predicted_value JSONB for display.
fn format_predicted_value(val: &Value) -> String {
    match val {
        Value::Number(n) => format!("{n}"),
        Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .take(3)
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            entries.join(", ")
        }
        Value::String(s) => s.clone(),
        _ => val.to_string(),
    }
}

/// Append a features summary section if features exist.
fn append_features_summary(parts: &mut Vec<String>, ctx: &ChatContext) {
    if ctx.features.is_empty() {
        return;
    }

    parts.push("\n### Latest Features\n".to_string());
    for feat in ctx.features.iter().take(3) {
        let fset = feat["feature_set"].as_str().unwrap_or("unknown");
        let source = feat["source"].as_str().unwrap_or("unknown");
        let computed = feat["computed_at"].as_str().unwrap_or("unknown");

        parts.push(format!("**{fset}** (source: {source}, at: {computed}):\n"));

        if let Some(features) = feat["features"].as_object() {
            for (key, val) in features.iter().take(6) {
                parts.push(format!("  - {key}: {val}\n"));
            }
        }
    }
}

/// Extract a specific feature value by searching keys that contain the given substring.
fn extract_feature_value(ctx: &ChatContext, key_substr: &str) -> Option<String> {
    for feat in &ctx.features {
        if let Some(features) = feat["features"].as_object() {
            for (key, val) in features {
                if key.to_lowercase().contains(key_substr) {
                    return Some(format!("{key}: {val}"));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::context::ChatContext;

    fn make_ctx(
        predictions: Vec<Value>,
        decisions: Vec<Value>,
        features: Vec<Value>,
    ) -> ChatContext {
        ChatContext {
            symbol: "BTCUSDT".to_string(),
            predictions,
            decisions,
            features,
            sources: vec!["ml_predictions".to_string()],
        }
    }

    #[test]
    fn test_why_buy_template() {
        let ctx = make_ctx(
            vec![serde_json::json!({
                "model_name": "catboost",
                "confidence": 0.85,
                "prediction_type": "price",
                "horizon": "1h",
                "predicted_value": {"direction": "up", "magnitude": 0.02}
            })],
            vec![serde_json::json!({
                "decision_type": "trade_signal",
                "confidence": 0.82,
                "explanation": "Strong bullish momentum detected"
            })],
            vec![],
        );
        let (answer, sources) = generate_response(&ctx, "Why did AI buy BTC?");
        assert!(answer.contains("Model Consensus"));
        assert!(answer.contains("catboost"));
        assert!(answer.contains("85.0%"));
        assert!(answer.contains("AI Decision Reasoning"));
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_risk_template() {
        let ctx = make_ctx(
            vec![serde_json::json!({
                "model_name": "garch",
                "confidence": 0.75,
                "prediction_type": "volatility",
                "horizon": "1d",
                "predicted_value": {"annualized_vol": 0.45}
            })],
            vec![],
            vec![serde_json::json!({
                "feature_set": "risk",
                "features": {"var_95": -0.035, "drawdown_max": -0.12},
                "computed_at": "2026-04-02T10:00:00Z",
                "source": "rust_native"
            })],
        );
        let (answer, _) = generate_response(&ctx, "What is the risk for BTC?");
        assert!(answer.contains("Risk Analysis"));
        assert!(answer.contains("garch"));
        assert!(answer.contains("var_95"));
    }

    #[test]
    fn test_performance_template() {
        let ctx = make_ctx(
            vec![
                serde_json::json!({
                    "model_name": "catboost",
                    "confidence": 0.85,
                    "prediction_type": "price",
                    "actual_value": {"price": 67000},
                    "error_metric": 0.012,
                    "is_fallback": false
                }),
                serde_json::json!({
                    "model_name": "finbert",
                    "confidence": 0.72,
                    "prediction_type": "sentiment",
                    "actual_value": null,
                    "is_fallback": false
                }),
            ],
            vec![],
            vec![],
        );
        let (answer, _) = generate_response(&ctx, "How is BTCUSDT performing?");
        assert!(answer.contains("Prediction Performance"));
        assert!(answer.contains("Total predictions (24h):** 2"));
        assert!(answer.contains("Resolved:** 1"));
    }

    #[test]
    fn test_general_fallback() {
        let ctx = make_ctx(vec![], vec![], vec![]);
        let (answer, _) = generate_response(&ctx, "Tell me about BTCUSDT");
        assert!(answer.contains("General Summary"));
        assert!(answer.contains("Tell me about BTCUSDT"));
        assert!(answer.contains("Try asking"));
    }

    #[test]
    fn test_empty_predictions_why_buy() {
        let ctx = make_ctx(vec![], vec![], vec![]);
        let (answer, _) = generate_response(&ctx, "Why buy BTC?");
        assert!(answer.contains("No recent predictions found"));
    }

    #[test]
    fn test_format_predicted_value_number() {
        let val = serde_json::json!(42.5);
        assert_eq!(format_predicted_value(&val), "42.5");
    }

    #[test]
    fn test_format_predicted_value_object() {
        let val = serde_json::json!({"direction": "up"});
        let formatted = format_predicted_value(&val);
        assert!(formatted.contains("direction"));
    }

    #[test]
    fn test_average_confidence() {
        let preds = vec![
            serde_json::json!({"confidence": 0.8}),
            serde_json::json!({"confidence": 0.6}),
        ];
        let avg = average_confidence(&preds);
        assert!((avg - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_average_confidence_empty() {
        let avg = average_confidence(&[]);
        assert_eq!(avg, 0.0);
    }
}
