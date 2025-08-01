//! Defines complex, multi-step, stateful tasks, such as lifecycle loops.

use crate::common::TaskId;
use std::time::Duration;

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
///
/// A `LifecycleLoop` subscribes to an interval and advances one step in its
/// sequence each time the interval fires. This allows for creating complex,
//  timed automations.
pub struct LifecycleLoop {
    pub task_id: TaskId,
    steps: Vec<LifecycleStep>,
    current_step: usize,
    repetition_policy: RepetitionPolicy,
    current_run_count: u32,
    interval: Duration,
}

impl LifecycleLoop {
    /// Creates a new `LifecycleLoop`.
    pub fn new(
        task_id: TaskId,
        steps: Vec<LifecycleStep>,
        repetition_policy: RepetitionPolicy,
        interval: Duration,
    ) -> Self {
        Self {
            task_id,
            steps,
            current_step: 0,
            repetition_policy,
            current_run_count: 0,
            interval,
        }
    }

    /// Processes a `TaskFired` event to advance the lifecycle's state.
    ///
    /// This method contains the core logic for executing the current step,
    /// advancing the step counter, handling repetitions, and determining
    /// if the lifecycle is complete.
    pub fn advance(&mut self) {
        // TODO: Implement the full logic for advancing the lifecycle.
        // 1. Check if there are any steps.
        // 2. Execute the current step: `self.steps[self.current_step]()`.
        // 3. Increment `self.current_step`.
        // 4. If at the end of the steps, check `repetition_policy`.
        // 5. Either loop, increment `run_count`, or mark as complete.
        // 6. Fire `AutomationEvent`s (StepAdvanced, Completed, Looped).
    }
}
