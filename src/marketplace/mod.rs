//! Copy Trading Marketplace Module (Wave 3 Task 17).
//!
//! Allows users to publish their strategies, browse public listings,
//! and subscribe to strategies created by other traders.

pub mod repository;

pub use repository::{
    list_public_strategies, publish_strategy, subscribe_to_strategy, PublishRequest,
    StrategyListing,
};
