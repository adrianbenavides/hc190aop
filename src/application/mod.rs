//! Application layer containing the core business logic orchestration.
//!
//! This module defines the `PaymentEngine` which acts as the primary entry point
//! for processing transactions. It uses an Actor-like pattern with `tokio` channels
//! to manage concurrency and state isolation.

pub mod engine;
