//! Defines complex, multi-step, stateful tasks, such as lifecycle loops.

use crate::common::{ListenerId, TaskId};
use crate::events::AutomationEvent;
use tokio::sync::broadcast;

/// A function closure that represents one step in a lifecycle.
pub type LifecycleStep = Box<dyn FnMut() + Send + Sync>;

/// Defines the repetition behavior of a `LifecycleLoop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepetitionPolicy {
    /// The lifecycle runs through its steps once and then completes.
    RunOnce,
    /// The lifecycle runs through its steps N times and then completes.
    RunNTimes(u32),
    /// The lifecycle repeats indefinitely.
    Repeat,
}

/// A stateful automation that executes a sequence of tasks in order.
#[doc(hidden)]
pub(crate) struct LifecycleLoop {
    pub id: TaskId,
    pub listener_id: ListenerId,
    steps: Vec<LifecycleStep>,
    current_step: usize,
    repetition_policy: RepetitionPolicy,
    run_count: u32,
}

impl LifecycleLoop {
    /// Creates a new `LifecycleLoop`.
    pub(crate) fn new(
        id: TaskId,
        listener_id: ListenerId,
        steps: Vec<LifecycleStep>,
        repetition_policy: RepetitionPolicy,
    ) -> Self {
        Self {
            id,
            listener_id,
            steps,
            current_step: 0,
            repetition_policy,
            run_count: 0,
        }
    }

    /// Processes a `TaskFired` event to advance the lifecycle's state.
    /// Returns `true` if the lifecycle has completed and should be removed.
    pub(crate) fn advance(
        &mut self,
        automation_event_sender: &broadcast::Sender<AutomationEvent>,
    ) -> bool {
        if self.steps.is_empty() {
            return true;
        }
        if let Some(step) = self.steps.get_mut(self.current_step) {
            (step)();
        }

        automation_event_sender
            .send(AutomationEvent::LifecycleStepAdvanced {
                id: self.id,
                step_index: self.current_step,
            })
            .ok();

        self.current_step += 1;

        if self.current_step >= self.steps.len() {
            self.run_count += 1;
            self.current_step = 0;

            match self.repetition_policy {
                RepetitionPolicy::RunOnce => {
                    automation_event_sender
                        .send(AutomationEvent::LifecycleCompleted { id: self.id })
                        .ok();
                    return true;
                }
                RepetitionPolicy::RunNTimes(n) => {
                    if self.run_count >= n {
                        automation_event_sender
                            .send(AutomationEvent::LifecycleCompleted { id: self.id })
                            .ok();
                        return true;
                    } else {
                        automation_event_sender
                            .send(AutomationEvent::LifecycleLooped { id: self.id })
                            .ok();
                    }
                }
                RepetitionPolicy::Repeat => {
                    automation_event_sender
                        .send(AutomationEvent::LifecycleLooped { id: self.id })
                        .ok();
                }
            }
        }
        false
    }
}
