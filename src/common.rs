//! Contains common, primitive types and a prelude for easy importing.
//!
//! This module defines the basic ID types used to uniquely identify phases, listeners,
//! tasks, and other components within the Hyperclock engine. Using distinct types
//! improves type safety and code clarity.

use serde::Deserialize;
use slotmap::new_key_type;

/// A prelude module for convenient importing of the most common Hyperclock types.
///
/// # Example
/// ```
/// use hyperclock::prelude::*;
/// ```
pub mod prelude {
    pub use super::{ListenerId, PhaseId, TaskId};
    pub use crate::config::HyperclockConfig;
    pub use crate::engine::HyperclockEngine;
}

new_key_type! {
    /// Uniquely and safely identifies a registered listener within the engine.
    ///
    /// This key is returned when a new listener (e.g., for an interval or a gong)
    /// is added to the engine. It is guaranteed to be unique and will not be reused,
    /// preventing stale ID bugs.
    pub struct ListenerId;

    /// Uniquely and safely identifies a stateful, complex task like a `LifecycleLoop`.
    pub struct TaskId;
}

/// Uniquely identifies a phase within the engine's cycle.
///
/// A `PhaseId` is a lightweight identifier, typically a small integer, that
/// corresponds to a step in the configured phase sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct PhaseId(pub u8);
