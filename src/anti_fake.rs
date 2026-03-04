//! Anti-fake protection layer for request authenticity and spoofing resistance.
//!
//! This module provides layered anti-fake controls:
//! - session fingerprint binding (IP + User-Agent)
//! - nonce replay protection
//! - request timestamp skew checks
//! - optional HMAC signature validation
//! - velocity limits for authenticated requests and login attempts

use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct AntiFakeConfig {
    pub enforce: bool,
    pub require_signature_for_writes: bool,
    pub enforce_real_data: bool,
    pub allow_demo_endpoints: bool,
    pub fake_endpoint_prefixes: Vec<String>,
    pub max_timestamp_skew_secs: i64,
    pub nonce_ttl_secs: i64,
    pub session_binding_ttl_secs: i64,
    pub max_session_bindings: usize,
    pub min_nonce_len: usize,
    pub max_nonce_len: usize,
    pub max_signature_len: usize,
    pub max_requests_per_minute: usize,
    pub max_failed_logins_per_minute: usize,
    pub hmac_secret: Option<String>,
}

impl Default for AntiFakeConfig {
    fn default() -> Self {
        Self {
            enforce: false,
            require_signature_for_writes: false,
            enforce_real_data: false,
            allow_demo_endpoints: false,
            fake_endpoint_prefixes: vec!["/api/demo".to_string()],
            max_timestamp_skew_secs: 90,
            nonce_ttl_secs: 300,
            session_binding_ttl_secs: 7 * 24 * 60 * 60,
            max_session_bindings: 100_000,
            min_nonce_len: 8,
            max_nonce_len: 128,
            max_signature_len: 256,
            max_requests_per_minute: 120,
            max_failed_logins_per_minute: 5,
            hmac_secret: None,
        }
    }
}

impl AntiFakeConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.enforce = std::env::var("ANTI_FAKE_ENFORCE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        cfg.require_signature_for_writes = std::env::var("ANTI_FAKE_REQUIRE_SIGNATURE_FOR_WRITES")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        cfg.enforce_real_data = std::env::var("ANTI_FAKE_ENFORCE_FAKE_DATA")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        cfg.allow_demo_endpoints = std::env::var("ANTI_FAKE_ALLOW_DEMO_ENDPOINTS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        cfg.fake_endpoint_prefixes = std::env::var("ANTI_FAKE_FAKE_ENDPOINT_PREFIXES")
            .ok()
            .map(Self::parse_path_prefixes)
            .unwrap_or_else(|| vec!["/api/demo".to_string()]);
        cfg.max_timestamp_skew_secs = std::env::var("ANTI_FAKE_MAX_TIMESTAMP_SKEW_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.max_timestamp_skew_secs);
        cfg.nonce_ttl_secs = std::env::var("ANTI_FAKE_NONCE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.nonce_ttl_secs);
        cfg.session_binding_ttl_secs = std::env::var("ANTI_FAKE_SESSION_BINDING_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.session_binding_ttl_secs);
        cfg.max_session_bindings = std::env::var("ANTI_FAKE_MAX_SESSION_BINDINGS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.max_session_bindings);
        cfg.min_nonce_len = std::env::var("ANTI_FAKE_MIN_NONCE_LEN")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.min_nonce_len);
        cfg.max_nonce_len = std::env::var("ANTI_FAKE_MAX_NONCE_LEN")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= cfg.min_nonce_len)
            .unwrap_or(cfg.max_nonce_len);
        cfg.max_signature_len = std::env::var("ANTI_FAKE_MAX_SIGNATURE_LEN")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v >= 64)
            .unwrap_or(cfg.max_signature_len);
        cfg.max_requests_per_minute = std::env::var("ANTI_FAKE_MAX_RPM")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.max_requests_per_minute);
        cfg.max_failed_logins_per_minute = std::env::var("ANTI_FAKE_MAX_FAILED_LOGINS_PER_MIN")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(cfg.max_failed_logins_per_minute);
        cfg.hmac_secret = std::env::var("ANTI_FAKE_HMAC_SECRET")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        // Production environment: enforce anti-fake by default
        if std::env::var("ENVIRONMENT").as_deref() == Ok("production") {
            if !cfg.enforce {
                cfg.enforce = true;
            }
            if !cfg.enforce_real_data {
                cfg.enforce_real_data = true;
            }
        }

        cfg
    }

    fn parse_path_prefixes(raw: String) -> Vec<String> {
        raw.split(',')
            .map(str::trim)
            .filter(|prefix| !prefix.is_empty())
            .map(|prefix| {
                let normalized = prefix.trim().to_lowercase();
                if normalized.starts_with('/') {
                    normalized
                } else {
                    format!("/{normalized}")
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct RequestAntiFakeSignal {
    pub method: String,
    pub path: String,
    pub access_token: String,
    pub client_ip: String,
    pub user_agent: String,
    pub request_timestamp_unix: Option<i64>,
    pub nonce: Option<String>,
    pub signature: Option<String>,
    pub payload_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AntiFakeDecision {
    Allow { score: u8, reasons: Vec<String> },
    Challenge { score: u8, reasons: Vec<String> },
    Block { score: u8, reasons: Vec<String> },
}

impl AntiFakeDecision {
    pub fn score(&self) -> u8 {
        match self {
            AntiFakeDecision::Allow { score, .. }
            | AntiFakeDecision::Challenge { score, .. }
            | AntiFakeDecision::Block { score, .. } => *score,
        }
    }

    pub fn reasons(&self) -> &[String] {
        match self {
            AntiFakeDecision::Allow { reasons, .. }
            | AntiFakeDecision::Challenge { reasons, .. }
            | AntiFakeDecision::Block { reasons, .. } => reasons,
        }
    }
}

#[derive(Debug)]
pub struct AntiFakeShield {
    config: AntiFakeConfig,
    session_fingerprints: HashMap<String, (String, DateTime<Utc>)>,
    seen_nonces: HashMap<String, DateTime<Utc>>,
    request_windows: HashMap<String, Vec<DateTime<Utc>>>,
    failed_login_windows: HashMap<String, Vec<DateTime<Utc>>>,
    data_policy_violation_count: u64,
    data_policy_violation_reasons: HashMap<String, u64>,
}

impl AntiFakeShield {
    pub fn new(config: AntiFakeConfig) -> Self {
        Self {
            config,
            session_fingerprints: HashMap::new(),
            seen_nonces: HashMap::new(),
            request_windows: HashMap::new(),
            failed_login_windows: HashMap::new(),
            data_policy_violation_count: 0,
            data_policy_violation_reasons: HashMap::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(AntiFakeConfig::from_env())
    }

    pub fn enforce_mode(&self) -> bool {
        self.config.enforce
    }

    pub fn enforce_real_data(&self) -> bool {
        self.config.enforce_real_data
    }

    pub fn allow_demo_endpoints(&self) -> bool {
        self.config.allow_demo_endpoints
    }

    pub fn fake_endpoint_prefixes(&self) -> Vec<String> {
        self.config.fake_endpoint_prefixes.clone()
    }

    pub fn data_policy_violation_count(&self) -> u64 {
        self.data_policy_violation_count
    }

    pub fn data_policy_violation_reasons(&self) -> Vec<(String, u64)> {
        self.data_policy_violation_reasons
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    pub fn evaluate_fake_data_policy(&mut self, path: &str) -> AntiFakeDecision {
        if !self.config.enforce_real_data || self.config.allow_demo_endpoints {
            return AntiFakeDecision::Allow {
                score: 0,
                reasons: Vec::new(),
            };
        }

        let path_lower = path.trim().to_lowercase();
        if self
            .config
            .fake_endpoint_prefixes
            .iter()
            .any(|prefix| path_lower.starts_with(prefix))
        {
            let reason = "blocked-fake-endpoint".to_string();
            self.data_policy_violation_count = self.data_policy_violation_count.saturating_add(1);
            *self
                .data_policy_violation_reasons
                .entry(reason.clone())
                .or_insert(0) += 1;

            AntiFakeDecision::Block {
                score: 100,
                reasons: vec![reason, format!("path-policy-prefix:{path_lower}")],
            }
        } else {
            AntiFakeDecision::Allow {
                score: 0,
                reasons: Vec::new(),
            }
        }
    }

    pub fn bind_session(&mut self, access_token: &str, client_ip: &str, user_agent: &str) {
        self.cleanup_state();
        let fp = Self::fingerprint(client_ip, user_agent);
        if self.session_fingerprints.len() >= self.config.max_session_bindings {
            self.prune_oldest_session_binding();
        }
        self.session_fingerprints
            .insert(access_token.to_string(), (fp, Utc::now()));
    }

    pub fn unbind_session(&mut self, access_token: &str) {
        self.session_fingerprints.remove(access_token);
    }

    pub fn evaluate_login_attempt(
        &mut self,
        principal: &str,
        client_ip: &str,
        user_agent: &str,
    ) -> AntiFakeDecision {
        self.cleanup_state();
        let now = Utc::now();
        let principal = principal.trim().to_lowercase();
        let ip = client_ip.trim();
        let principal_key = format!("principal:{principal}|{ip}");
        let events = self.failed_login_windows.entry(principal_key).or_default();
        events.retain(|ts| *ts > now - Duration::minutes(1));
        let principal_attempts = events.len();

        let mut ip_attempts = 0usize;
        if !ip.is_empty() && ip != "unknown" {
            let ip_key = format!("ip:{ip}");
            let ip_events = self.failed_login_windows.entry(ip_key).or_default();
            ip_events.retain(|ts| *ts > now - Duration::minutes(1));
            ip_attempts = ip_events.len();
        }

        let mut score: u8 = 0;
        let mut reasons = Vec::new();
        if user_agent.trim().is_empty() || user_agent == "unknown" {
            score = score.saturating_add(15);
            reasons.push("missing-or-unknown-user-agent".to_string());
        }
        if principal_attempts >= self.config.max_failed_logins_per_minute {
            score = score.saturating_add(70);
            reasons.push("failed-login-velocity-threshold-exceeded-principal".to_string());
        }
        let ip_limit = self.config.max_failed_logins_per_minute.saturating_mul(3);
        if ip_attempts >= ip_limit {
            score = score.saturating_add(70);
            reasons.push("failed-login-velocity-threshold-exceeded-ip".to_string());
        }

        self.decision_from_score(score, reasons)
    }

    pub fn record_failed_login(&mut self, principal: &str, client_ip: &str) {
        self.cleanup_state();
        let principal = principal.trim().to_lowercase();
        let ip = client_ip.trim();
        let key = format!("principal:{principal}|{ip}");
        self.failed_login_windows
            .entry(key)
            .or_default()
            .push(Utc::now());
        if !ip.is_empty() && ip != "unknown" {
            self.failed_login_windows
                .entry(format!("ip:{ip}"))
                .or_default()
                .push(Utc::now());
        }
    }

    pub fn evaluate_authenticated_request(
        &mut self,
        signal: RequestAntiFakeSignal,
    ) -> AntiFakeDecision {
        self.cleanup_state();
        let now = Utc::now();
        let mut score: u8 = 0;
        let mut reasons = Vec::new();
        let is_write = matches!(signal.method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE");

        if signal.user_agent.trim().is_empty() || signal.user_agent == "unknown" {
            score = score.saturating_add(10);
            reasons.push("missing-or-unknown-user-agent".to_string());
        }

        if signal.client_ip.trim().is_empty() {
            score = score.saturating_add(10);
            reasons.push("missing-client-ip".to_string());
        }

        if let Some((bound_fp, _)) = self.session_fingerprints.get(&signal.access_token) {
            let current_fp = Self::fingerprint(&signal.client_ip, &signal.user_agent);
            if bound_fp != &current_fp {
                score = score.saturating_add(45);
                reasons.push("session-fingerprint-mismatch".to_string());
            }
        } else {
            score = score.saturating_add(8);
            reasons.push("session-fingerprint-not-bound".to_string());
        }

        if let Some(ts) = signal.request_timestamp_unix {
            if let Some(req_time) = DateTime::from_timestamp(ts, 0) {
                let skew = (now - req_time).num_seconds().unsigned_abs() as i64;
                if skew > self.config.max_timestamp_skew_secs {
                    score = score.saturating_add(35);
                    reasons.push("request-timestamp-skew-too-high".to_string());
                }
            } else {
                score = score.saturating_add(35);
                reasons.push("invalid-request-timestamp".to_string());
            }
        } else if is_write {
            score = score.saturating_add(20);
            reasons.push("missing-request-timestamp".to_string());
        }

        if let Some(nonce) = signal.nonce.as_ref() {
            if !self.valid_nonce(nonce) {
                score = score.saturating_add(60);
                reasons.push("invalid-request-nonce-format".to_string());
            } else {
                let nonce_key = format!("{}|{}", signal.access_token, nonce);
                if self.seen_nonces.contains_key(&nonce_key) {
                    score = score.saturating_add(80);
                    reasons.push("replay-nonce-detected".to_string());
                } else {
                    self.seen_nonces.insert(nonce_key, now);
                }
            }
        } else if is_write {
            score = score.saturating_add(25);
            reasons.push("missing-request-nonce".to_string());
        }

        let token_velocity_key = format!("token:{}", signal.access_token);
        let token_count = self.record_and_count_window(&token_velocity_key, now);
        if token_count > self.config.max_requests_per_minute {
            score = score.saturating_add(50);
            reasons.push("request-velocity-threshold-exceeded-token".to_string());
        }

        if signal.client_ip != "unknown" && !signal.client_ip.is_empty() {
            let ip_velocity_key = format!("ip:{}", signal.client_ip);
            let ip_count = self.record_and_count_window(&ip_velocity_key, now);
            if ip_count > self.config.max_requests_per_minute.saturating_mul(2) {
                score = score.saturating_add(35);
                reasons.push("request-velocity-threshold-exceeded-ip".to_string());
            }
        }

        if let Some(sig) = signal.signature.as_ref() {
            if sig.trim().len() > self.config.max_signature_len {
                score = score.saturating_add(80);
                reasons.push("signature-format-invalid".to_string());
            }
            let signature_valid = self
                .verify_signature(
                    &signal.method,
                    &signal.path,
                    signal.request_timestamp_unix,
                    signal.nonce.as_deref(),
                    signal.payload_hash.as_deref(),
                    sig,
                )
                .unwrap_or(false);
            if !signature_valid {
                score = score.saturating_add(80);
                reasons.push("invalid-request-signature".to_string());
            }
        } else if is_write && self.config.require_signature_for_writes {
            score = score.saturating_add(40);
            reasons.push("missing-signature-for-write-request".to_string());
        }

        self.decision_from_score(score, reasons)
    }

    fn decision_from_score(&self, score: u8, reasons: Vec<String>) -> AntiFakeDecision {
        if score >= 70 {
            AntiFakeDecision::Block { score, reasons }
        } else if score >= 35 {
            AntiFakeDecision::Challenge { score, reasons }
        } else {
            AntiFakeDecision::Allow { score, reasons }
        }
    }

    fn verify_signature(
        &self,
        method: &str,
        path: &str,
        timestamp_unix: Option<i64>,
        nonce: Option<&str>,
        payload_hash: Option<&str>,
        provided_signature: &str,
    ) -> Option<bool> {
        let secret = self.config.hmac_secret.as_ref()?;
        let ts = timestamp_unix?;
        let nonce = nonce.unwrap_or_default();
        let payload_hash = payload_hash.unwrap_or_default();
        let canonical = format!("{method}\n{path}\n{ts}\n{nonce}\n{payload_hash}");
        let provided_signature = provided_signature.trim();
        let provided_bytes = Self::decode_hex(provided_signature)?;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
        mac.update(canonical.as_bytes());
        Some(mac.verify_slice(&provided_bytes).is_ok())
    }

    fn to_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }

    fn decode_hex(input: &str) -> Option<Vec<u8>> {
        if input.is_empty() || input.len() % 2 != 0 {
            return None;
        }
        let mut out = Vec::with_capacity(input.len() / 2);
        let bytes = input.as_bytes();
        let mut idx = 0usize;
        while idx < bytes.len() {
            let hi = Self::hex_value(bytes[idx])?;
            let lo = Self::hex_value(bytes[idx + 1])?;
            out.push((hi << 4) | lo);
            idx += 2;
        }
        Some(out)
    }

    fn hex_value(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    fn fingerprint(client_ip: &str, user_agent: &str) -> String {
        format!(
            "{}|{}",
            client_ip.trim().to_lowercase(),
            user_agent.trim().to_lowercase()
        )
    }

    fn valid_nonce(&self, nonce: &str) -> bool {
        let len = nonce.len();
        if len < self.config.min_nonce_len || len > self.config.max_nonce_len {
            return false;
        }
        nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    }

    fn record_and_count_window(&mut self, key: &str, now: DateTime<Utc>) -> usize {
        let reqs = self.request_windows.entry(key.to_string()).or_default();
        reqs.retain(|ts| *ts > now - Duration::minutes(1));
        reqs.push(now);
        reqs.len()
    }

    fn prune_oldest_session_binding(&mut self) {
        let oldest = self
            .session_fingerprints
            .iter()
            .min_by_key(|(_, (_, ts))| *ts)
            .map(|(k, _)| k.clone());
        if let Some(key) = oldest {
            self.session_fingerprints.remove(&key);
        }
    }

    fn cleanup_state(&mut self) {
        let now = Utc::now();
        let session_cutoff = now - Duration::seconds(self.config.session_binding_ttl_secs);
        self.session_fingerprints
            .retain(|_, (_, ts)| *ts > session_cutoff);

        let nonce_cutoff = now - Duration::seconds(self.config.nonce_ttl_secs);
        self.seen_nonces.retain(|_, ts| *ts > nonce_cutoff);

        let window_cutoff = now - Duration::minutes(1);
        for values in self.request_windows.values_mut() {
            values.retain(|ts| *ts > window_cutoff);
        }
        self.request_windows.retain(|_, values| !values.is_empty());

        for values in self.failed_login_windows.values_mut() {
            values.retain(|ts| *ts > window_cutoff);
        }
        self.failed_login_windows
            .retain(|_, values| !values.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_signal() -> RequestAntiFakeSignal {
        RequestAntiFakeSignal {
            method: "POST".to_string(),
            path: "/api/broker/orders".to_string(),
            access_token: "atk_test".to_string(),
            client_ip: "127.0.0.1".to_string(),
            user_agent: "investor-os-tests".to_string(),
            request_timestamp_unix: Some(Utc::now().timestamp()),
            nonce: Some("nonce-0001".to_string()),
            signature: None,
            payload_hash: Some("abc123".to_string()),
        }
    }

    #[test]
    fn test_replay_nonce_detection_blocks() {
        let mut shield = AntiFakeShield::new(AntiFakeConfig::default());
        shield.bind_session("atk_test", "127.0.0.1", "investor-os-tests");

        let first = shield.evaluate_authenticated_request(base_signal());
        assert!(matches!(
            first,
            AntiFakeDecision::Allow { .. } | AntiFakeDecision::Challenge { .. }
        ));

        let replay = shield.evaluate_authenticated_request(base_signal());
        assert!(matches!(replay, AntiFakeDecision::Block { .. }));
        assert!(replay
            .reasons()
            .iter()
            .any(|r| r == "replay-nonce-detected"));
    }

    #[test]
    fn test_failed_login_velocity_blocks() {
        let mut cfg = AntiFakeConfig::default();
        cfg.max_failed_logins_per_minute = 2;
        let mut shield = AntiFakeShield::new(cfg);

        shield.record_failed_login("admin@investor-os.com", "127.0.0.1");
        shield.record_failed_login("admin@investor-os.com", "127.0.0.1");

        let decision = shield.evaluate_login_attempt(
            "admin@investor-os.com",
            "127.0.0.1",
            "investor-os-tests",
        );
        assert!(matches!(decision, AntiFakeDecision::Block { .. }));
    }

    #[test]
    fn test_signature_validation_paths() {
        let mut cfg = AntiFakeConfig::default();
        cfg.hmac_secret = Some("super-secret".to_string());
        cfg.require_signature_for_writes = true;
        let mut shield = AntiFakeShield::new(cfg);
        shield.bind_session("atk_test", "127.0.0.1", "investor-os-tests");

        let mut signal = base_signal();
        signal.nonce = Some("nonce-sign".to_string());
        signal.request_timestamp_unix = Some(Utc::now().timestamp());
        let canonical = format!(
            "{}\n{}\n{}\n{}\n{}",
            signal.method,
            signal.path,
            signal.request_timestamp_unix.unwrap(),
            signal.nonce.clone().unwrap(),
            signal.payload_hash.clone().unwrap()
        );

        let mut mac = HmacSha256::new_from_slice("super-secret".as_bytes()).unwrap();
        mac.update(canonical.as_bytes());
        let sig = AntiFakeShield::to_hex(&mac.finalize().into_bytes());
        signal.signature = Some(sig);

        let decision = shield.evaluate_authenticated_request(signal);
        assert!(matches!(decision, AntiFakeDecision::Allow { .. }));
    }

    #[test]
    fn test_invalid_nonce_format_is_rejected() {
        let mut shield = AntiFakeShield::new(AntiFakeConfig::default());
        shield.bind_session("atk_test", "127.0.0.1", "investor-os-tests");

        let mut signal = base_signal();
        signal.nonce = Some("x".to_string());
        let decision = shield.evaluate_authenticated_request(signal);
        assert!(matches!(
            decision,
            AntiFakeDecision::Challenge { .. } | AntiFakeDecision::Block { .. }
        ));
        assert!(decision
            .reasons()
            .iter()
            .any(|r| r == "invalid-request-nonce-format"));
    }

    #[test]
    fn test_invalid_signature_hex_blocks() {
        let mut cfg = AntiFakeConfig::default();
        cfg.require_signature_for_writes = true;
        cfg.hmac_secret = Some("super-secret".to_string());
        let mut shield = AntiFakeShield::new(cfg);
        shield.bind_session("atk_test", "127.0.0.1", "investor-os-tests");

        let mut signal = base_signal();
        signal.signature = Some("not-hex".to_string());
        let decision = shield.evaluate_authenticated_request(signal);
        assert!(matches!(decision, AntiFakeDecision::Block { .. }));
        assert!(decision
            .reasons()
            .iter()
            .any(|r| r == "invalid-request-signature"));
    }

    #[test]
    fn test_session_binding_ttl_cleanup() {
        let mut cfg = AntiFakeConfig::default();
        cfg.session_binding_ttl_secs = 1;
        let mut shield = AntiFakeShield::new(cfg);
        shield.session_fingerprints.insert(
            "atk_old".to_string(),
            (
                "127.0.0.1|old-agent".to_string(),
                Utc::now() - Duration::seconds(10),
            ),
        );

        let _ = shield.evaluate_login_attempt("admin@investor-os.com", "127.0.0.1", "agent");
        assert!(!shield.session_fingerprints.contains_key("atk_old"));
    }

    #[test]
    fn test_fake_data_policy_blocks_demo_endpoints() {
        let mut cfg = AntiFakeConfig::default();
        cfg.enforce_real_data = true;
        let mut shield = AntiFakeShield::new(cfg);

        let decision = shield.evaluate_fake_data_policy("/api/demo/trade");
        assert!(matches!(decision, AntiFakeDecision::Block { .. }));
        assert_eq!(shield.data_policy_violation_count(), 1);
        assert!(shield
            .data_policy_violation_reasons()
            .iter()
            .any(|(reason, _)| {
                reason == "blocked-fake-endpoint" || reason.starts_with("path-policy-prefix:")
            }));
    }

    #[test]
    fn test_fake_data_policy_allows_demo_when_exception_is_enabled() {
        let mut cfg = AntiFakeConfig::default();
        cfg.enforce_real_data = true;
        cfg.allow_demo_endpoints = true;
        let mut shield = AntiFakeShield::new(cfg);

        let decision = shield.evaluate_fake_data_policy("/api/demo/trade");
        assert!(matches!(decision, AntiFakeDecision::Allow { .. }));
        assert_eq!(shield.data_policy_violation_count(), 0);
    }

    #[test]
    fn test_fake_data_policy_custom_prefixes() {
        let mut cfg = AntiFakeConfig::default();
        cfg.enforce_real_data = true;
        cfg.fake_endpoint_prefixes = vec!["/sandbox".to_string(), "/staging".to_string()];
        let mut shield = AntiFakeShield::new(cfg);

        assert!(matches!(
            shield.evaluate_fake_data_policy("/sandbox/positions"),
            AntiFakeDecision::Block { .. }
        ));
        assert!(matches!(
            shield.evaluate_fake_data_policy("/api/demo/trade"),
            AntiFakeDecision::Allow { .. }
        ));
        assert_eq!(shield.data_policy_violation_count(), 1);
    }
}
