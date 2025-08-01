//! # Hyperclock
//!
//! A high-performance, event-driven, phased time engine for Rust.
//!
//! Hyperclock provides the core engine for time-based, phased event processing.
//! It is designed to be a library that an application uses to manage complex,
//! time-sensitive logic in a structured and decoupled way.
//!
//! ## Core Concepts
//!
//! - **SystemClock**: A high-frequency ticker that acts as the single source of time.
//! - **Phased Cycle**: On every tick, the engine executes a configurable sequence of
//!   phases (e.g., "observe", "decide", "act"), allowing for structured logic
//!   that mirrors cognitive or industrial processes.
//! - **Event-Driven**: All logic is executed in response to strongly-typed events.
//!   Your application subscribes to event streams (`GongEvent`, `TaskEvent`, etc.)
//!   to perform work.
//! - **Configuration-Driven**: The engine's speed, phase sequence, and calendar
//!   events are defined at startup via a `HyperclockConfig` object, often loaded
//!   from a file.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use hyperclock::prelude::*;
//! use std::time::Duration;
//! use tokio::sync::broadcast;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 1. Create a default configuration.
//!     let config = HyperclockConfig::default();
//!
//!     // 2. Create the engine.
//!     let engine = HyperclockEngine::new(config);
//!
//!     // 3. Subscribe to an event stream before starting the engine.
//!     let mut system_events = engine.subscribe_system_events();
//!     tokio::spawn(async move {
//!         while let Ok(event) = system_events.recv().await {
//!             println!("Received System Event: {:?}", event);
//!         }
//!     });
//!
//!     // 4. Register listeners.
//!     let _listener_id = engine.on_interval(
//!         PhaseId(0),
//!         Duration::from_secs(5),
//!         || println!("5 seconds have passed in phase 0!")
//!     ).await;
//!
//!     // 5. Run the engine. It will shut down on Ctrl+C.
//!     engine.run().await?;
//!
//!     Ok(())
//! }
//! ```

pub const ENGINE_NAME: &str = "Hyper Engine";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


// Declare all the modules in the crate.
pub mod common;
pub mod components;
pub mod config;
pub mod engine;
pub mod events;
pub mod time;

/// A prelude module for easy importing of the most common Hyperclock types.
pub mod prelude {
    pub use crate::common::{ListenerId, PhaseId, TaskId};
    pub use crate::config::{ClockResolution, HyperclockConfig};
    pub use crate::engine::HyperclockEngine;
    pub use crate::events::{
        AutomationEvent, ConditionalEvent, GongEvent, PhaseEvent, SystemEvent, TaskEvent,
        UserEvent,
    };
}

// A temporary default implementation for the config for the example.
impl Default for config::HyperclockConfig {
    fn default() -> Self {
        Self {
            resolution: config::ClockResolution::Low,
            phases: vec![config::PhaseConfig {
                id: common::PhaseId(0),
                label: "default".to_string(),
            }],
            gong_config: config::GongConfig::default(),
        }
    }
}
