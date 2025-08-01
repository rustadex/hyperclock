//! Defines all public event types broadcast by the Hyperclock engine.
//!
//! This module acts as the public API for the engine's event system. Listeners
//! subscribe to these specific, strongly-typed events to perform their work.

use crate::common::{ListenerId, PhaseId, TaskId};
use crate::time::TickEvent;
use chrono::{Date, NaiveTime, Utc};
use std::any::Any;
use std::sync::Arc;
use tokio::time::Instant;

/// The primary event that drives the engine's internal cycle.
///
/// It is fired for each configured phase within a single master `TickEvent`.
#[derive(Debug, Clone)]
pub struct PhaseEvent {
    /// The ID of the phase that is currently active.
    pub phase: PhaseId,
    /// A shared pointer to the master tick this phase belongs to.
    pub tick: Arc<TickEvent>,
}

/// Events related to the lifecycle and state of the engine itself.
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Fired once when the engine's `run` loop begins.
    EngineStarted { timestamp: Instant },
    /// Fired once when the engine's `run` loop is about to exit.
    EngineShutdown,
    /// Fired when a new listener is successfully added to the engine.
    ListenerAdded { id: ListenerId },
    /// Fired when a listener is removed from the engine.
    ListenerRemoved { id: ListenerId },
}

/// High-level, human-meaningful calendar and time-of-day events.
#[derive(Debug, Clone)]
pub enum GongEvent {
    /// Fired once when the calendar date changes.
    DateChanged { new_date: Date<Utc> },
    /// Fired at specific, named times of the day.
    TimeOfDay(TimeOfDay),
    /// Fired at specific, named workday milestones.
    WorkdayMilestone(WorkdayMilestone),
    /// Fired on a specific holiday defined in the `GongConfig`.
    Holiday { name: String, date: Date<Utc> },
}

/// A named time of day for `GongEvent`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOfDay {
    Noon,
    Midnight,
}

/// A named workday milestone for `GongEvent`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkdayMilestone {
    pub label: &'static str,
    pub time: NaiveTime,
}

/// Events related to the lifecycle of scheduled, repeating tasks.
#[derive(Debug, Clone)]
pub enum TaskEvent {
    /// Fired when a new task is successfully registered with the engine.
    TaskScheduled { id: TaskId },
    /// Fired each time a scheduled task's logic is executed.
    TaskFired { id: TaskId, tick: Arc<TickEvent> },
    /// Fired when a finite task has completed all its runs.
    TaskCompleted { id: TaskId },
}

/// Events related to the `LifecycleLoop` automation component.
#[derive(Debug, Clone)]
pub enum AutomationEvent {
    /// Fired when a new lifecycle loop is started.
    LifecycleStarted { id: TaskId },
    /// Fired each time the lifecycle advances to a new step.
    LifecycleStepAdvanced { id: TaskId, step_index: usize },
    /// Fired when a finite lifecycle has completed all its steps and repetitions.
    LifecycleCompleted { id: TaskId },
    /// Fired when a repeating lifecycle finishes its last step and starts over.
    LifecycleLooped { id: TaskId },
}

/// Fired when a registered condition is met.
#[derive(Debug, Clone)]
pub struct ConditionalEvent {
    pub condition_id: ListenerId,
    pub timestamp: Instant,
}

/// A generic container for custom, application-defined events.
///
/// This allows the application to use Hyperclock's event bus for its own messaging,
/// decoupling different parts of the application's logic.
#[derive(Debug)]
pub struct UserEvent {
    /// The name or type of the event, used for filtering.
    pub name: String,
    /// The type-erased data payload. The receiver is responsible for
    /// downcasting this to the expected concrete type.
    pub payload: Box<dyn Any + Send + Sync>,
}
