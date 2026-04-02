//! NATS client with JetStream support (Sprint N1).

use async_nats::jetstream;
use tracing::{error, info, warn};

/// NATS client wrapping connection + JetStream context.
#[derive(Clone)]
pub struct NatsClient {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    url: String,
}

impl NatsClient {
    /// Connect to NATS server.
    pub async fn connect(url: &str) -> Result<Self, async_nats::Error> {
        info!("Connecting to NATS at {url}...");

        let client = async_nats::ConnectOptions::new()
            .retry_on_initial_connect()
            .connect(url)
            .await?;

        let jetstream = jetstream::new(client.clone());

        info!("NATS connected to {url}");

        Ok(Self {
            client,
            jetstream,
            url: url.to_string(),
        })
    }

    /// Create client from NATS_URL environment variable. Returns None if not set.
    pub async fn from_env() -> Option<Self> {
        let url = std::env::var("NATS_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        match Self::connect(&url).await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("NATS connection failed (non-fatal): {e}");
                None
            }
        }
    }

    /// Get the JetStream context for stream operations.
    pub fn jetstream(&self) -> &jetstream::Context {
        &self.jetstream
    }

    /// Get the raw NATS client.
    pub fn client(&self) -> &async_nats::Client {
        &self.client
    }

    /// Publish a message to a subject.
    pub async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), async_nats::Error> {
        self.client
            .publish(
                async_nats::Subject::from(subject.to_string()),
                payload.into(),
            )
            .await
            .map_err(|e| {
                error!("NATS publish to {subject} failed: {e}");
                e.into()
            })
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.client.connection_state() == async_nats::connection::State::Connected
    }

    /// Get the URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}
