//! Contains the building blocks for creating time-based logic.
//!
//! This module provides concrete implementations for watchers (which react to
//! time passing) and tasks (which represent complex, multi-step automations).
//! The `HyperclockEngine` manages collections of these components to drive
//! the application's logic.

pub mod task;
pub mod watcher;
