//! Billing & Feature Gating Module (Wave 1b Task 7).
//!
//! Subscription tier definitions (Free / Pro / Enterprise),
//! feature gating logic, and PostgreSQL persistence.
//! Stripe webhook handling is future work — this module covers
//! the data model and gating functions only.

pub mod fiat_onramp;
pub mod repository;
pub mod tiers;
