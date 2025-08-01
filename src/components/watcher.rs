//! Defines watchers that react to the event stream to produce higher-level events.

use crate::common::{ListenerId, PhaseId};
use crate::config::GongConfig;
use crate::events::{ConditionalEvent, GongEvent, TaskEvent, TimeOfDay};
use crate::time::TickEvent;
use chrono::{Datelike, Timelike, Utc};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// A function closure that represents a condition to be checked.
pub type ConditionCheck = Box<dyn Fn() -> bool + Send + Sync>;

/// Watches a phase stream and fires a `TaskEvent` at a regular interval.
#[doc(hidden)]
pub(crate) struct IntervalWatcher {
    task_id: TaskId,
    phase_to_watch: PhaseId,
    interval: Duration,
    last_fired: Instant,
    task_logic: Box<dyn FnMut() + Send + Sync>,
}

impl IntervalWatcher {
    /// Creates a new `IntervalWatcher`.
    pub(crate) fn new(
        task_id: TaskId,
        phase_to_watch: PhaseId,
        interval: Duration,
        task_logic: Box<dyn FnMut() + Send + Sync>,
    ) -> Self {
        Self {
            task_id,
            phase_to_watch,
            interval,
            last_fired: Instant::now(),
            task_logic,
        }
    }

    /// Processes a phase event and executes its internal logic if the interval has elapsed.
    pub(crate) fn process_phase(&mut self, current_phase: PhaseId) {
        if current_phase == self.phase_to_watch {
            if self.last_fired.elapsed() >= self.interval {
                (self.task_logic)();
                self.last_fired = Instant::now();
                // Note: The engine itself will be responsible for firing the TaskFired event.
            }
        }
    }
}

/// Watches the clock for significant, human-meaningful calendar events.
#[doc(hidden)]
pub(crate) struct GongWatcher {
    config: Arc<GongConfig>,
    last_known_date: chrono::Date<chrono_tz::Tz>,
}

impl GongWatcher {
    /// Creates a new `GongWatcher`.
    pub(crate) fn new(config: Arc<GongConfig>) -> Self {
        let now = Utc::now().with_timezone(&config.timezone);
        Self {
            config,
            last_known_date: now.date(),
        }
    }

    /// Processes a tick and fires `GongEvent`s if calendar milestones have been crossed.
    pub(crate) fn process_tick(
        &mut self,
        _tick: &TickEvent,
        gong_event_sender: &broadcast::Sender<GongEvent>,
    ) {
        let now = Utc::now().with_timezone(&self.config.timezone);
        let current_date = now.date();

        // Check for date change, which is the most significant gong.
        if current_date != self.last_known_date {
            gong_event_sender
                .send(GongEvent::DateChanged {
                    new_date: current_date.with_timezone(&Utc),
                })
                .ok();

            // When the date changes, check if the new date is a configured holiday.
            for holiday in &self.config.holidays {
                if holiday.date == current_date.with_timezone(&Utc) {
                    gong_event_sender
                        .send(GongEvent::Holiday {
                            name: holiday.name.clone(),
                            date: holiday.date,
                        })
                        .ok();
                }
            }
            self.last_known_date = current_date;
        }

        // TODO: Implement checks for Noon, Midnight, Hourly, etc., by comparing
        // the current time to the time of the last known tick to fire only once.
    }
}

/// Watches for a specific condition to become true.
#[doc(hidden)]
pub(crate) struct ConditionalWatcher {
    condition: ConditionCheck,
    task_logic: Box<dyn FnMut() + Send + Sync>,
    is_one_shot: bool,
}

impl ConditionalWatcher {
    /// Creates a new `ConditionalWatcher`.
    pub(crate) fn new(
        condition: ConditionCheck,
        task_logic: Box<dyn FnMut() + Send + Sync>,
        is_one_shot: bool,
    ) -> Self {
        Self {
            condition,
            task_logic,
            is_one_shot,
        }
    }

    /// Executes the condition check and, if true, executes its internal logic.
    /// Returns `true` if the condition was met.
    pub(crate) fn check_and_fire(&mut self) -> bool {
        if (self.condition)() {
            (self.task_logic)();
            true
        } else {
            false
        }
    }
}
