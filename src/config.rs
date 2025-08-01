//! Defines all configuration structures for the Hyperclock engine.
//!
//! These structs are designed to be deserialized from a configuration file
//! (e.g., a TOML file) using `serde`. This allows the engine's behavior,
//! including its speed, phase sequence, and calendar events, to be defined
//! externally from the application code.

use crate::common::PhaseId;
use chrono::{Date, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

/// The top-level configuration for the `HyperclockEngine`.
///
/// This struct is the entry point for all engine settings. It is typically
/// loaded from a TOML or JSON file at application startup.
#[derive(Debug, Clone, Deserialize)]
pub struct HyperclockConfig {
    /// The tick speed of the master `SystemClock`.
    pub resolution: ClockResolution,

    /// Defines the sequence of phases to execute on each tick.
    /// The engine will iterate through this vector in order.
    #[serde(default = "default_phases")]
    pub phases: Vec<PhaseConfig>,

    /// Configuration for calendar-based "Gong" events.
    #[serde(default)]
    pub gong_config: GongConfig,
}

/// Defines the operational speed of the `SystemClock`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockResolution {
    /// ~60 ticks per second. Suitable for real-time applications.
    High,
    /// ~30 ticks per second. Suitable for general purpose simulations.
    Medium,
    /// ~1 tick per second. Suitable for strategic or turn-based logic.
    Low,
    /// A user-defined speed in ticks per second.
    Custom { ticks_per_second: u64 },
}

/// Defines a single phase in the engine's cycle.
#[derive(Debug, Clone, Deserialize)]
pub struct PhaseConfig {
    /// The numeric ID for this phase. Listeners will subscribe to this ID.
    pub id: PhaseId,
    /// A human-readable label for debugging and logging purposes.
    pub label: String,
}

/// Configuration for calendar-based `GongEvent`s.
#[derive(Debug, Clone, Deserialize)]
pub struct GongConfig {
    /// The timezone the engine should operate in for calendar calculations.
    /// Defaults to UTC. Uses the string names from the IANA Time Zone Database
    /// (e.g., "America/New_York").
    #[serde(default = "default_timezone")]
    pub timezone: Tz,

    /// A list of custom holidays the GongWatcher should be aware of.
    #[serde(default)]
    pub holidays: Vec<Holiday>,

    /// A list of custom times for workday-related gongs.
    #[serde(default)]
    pub workday_milestones: Vec<NaiveTime>,
}

/// Represents a custom holiday for the `GongWatcher`.
#[derive(Debug, Clone, Deserialize)]
pub struct Holiday {
    pub name: String,
    pub date: Date<Utc>,
}

// --- Default value functions for serde ---

fn default_phases() -> Vec<PhaseConfig> {
    vec![PhaseConfig {
        id: PhaseId(0),
        label: "default_phase".to_string(),
    }]
}

fn default_timezone() -> Tz {
    Tz::UTC
}

impl Default for GongConfig {
    fn default() -> Self {
        Self {
            timezone: default_timezone(),
            holidays: Vec::new(),
            workday_milestones: Vec::new(),
        }
    }
}
