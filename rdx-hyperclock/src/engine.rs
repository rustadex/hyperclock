//! The core engine that orchestrates the entire Hyperclock system.

use crate::common::{ListenerId, PhaseId, TaskId};
use crate::components::task::{LifecycleLoop, LifecycleStep, RepetitionPolicy};
use crate::components::watcher::{ConditionalWatcher, GongWatcher, IntervalWatcher};
use crate::config::HyperclockConfig;
use crate::events::{
    AutomationEvent, ConditionalEvent, GongEvent, PhaseEvent, SystemEvent, TaskEvent, UserEvent,
};
use crate::time::{SystemClock, TickEvent};
use slotmap::SlotMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, trace};

/// The main Hyperclock engine.
///
/// This struct is the central point of control. It holds the system's configuration,
/// manages all active listeners and tasks, and drives the event loop. The `Engine`
/// is designed to be cloned and shared across tasks, providing a handle to the
/// running instance.
#[derive(Clone)]
pub struct HyperclockEngine {
    config: Arc<HyperclockConfig>,
    tick_sender: broadcast::Sender<Arc<TickEvent>>,
    phase_sender: broadcast::Sender<PhaseEvent>,
    system_event_sender: broadcast::Sender<SystemEvent>,
    gong_event_sender: broadcast::Sender<GongEvent>,
    task_event_sender: broadcast::Sender<TaskEvent>,
    automation_event_sender: broadcast::Sender<AutomationEvent>,
    conditional_event_sender: broadcast::Sender<ConditionalEvent>,
    user_event_sender: broadcast::Sender<UserEvent>,
    interval_watchers: Arc<RwLock<SlotMap<ListenerId, IntervalWatcher>>>,
    gong_watchers: Arc<RwLock<SlotMap<ListenerId, GongWatcher>>>,
    conditional_watchers: Arc<RwLock<SlotMap<ListenerId, ConditionalWatcher>>>,
    lifecycle_loops: Arc<RwLock<SlotMap<TaskId, LifecycleLoop>>>,
    lifecycle_triggers: Arc<RwLock<HashMap<ListenerId, TaskId>>>,
}

// Core implementation block for internal logic.
impl HyperclockEngine {
    /// Creates a new `HyperclockEngine` with the given configuration.
    pub fn new(config: HyperclockConfig) -> Self {
        const CHANNEL_CAPACITY: usize = 256;
        let (tick_sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        let (phase_sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        let (system_event_sender, _) = broadcast::channel(64);
        let (gong_event_sender, _) = broadcast::channel(64);
        let (task_event_sender, _) = broadcast::channel(64);
        let (automation_event_sender, _) = broadcast::channel(64);
        let (conditional_event_sender, _) = broadcast::channel(64);
        let (user_event_sender, _) = broadcast::channel(64);

        let config_arc = Arc::new(config);
        let gong_config_arc = Arc::new(config_arc.gong_config.clone());
        let mut gong_watchers = SlotMap::with_key();
        gong_watchers.insert(GongWatcher::new(gong_config_arc));

        Self {
            config: config_arc,
            tick_sender,
            phase_sender,
            system_event_sender,
            gong_event_sender,
            task_event_sender,
            automation_event_sender,
            conditional_event_sender,
            user_event_sender,
            interval_watchers: Arc::new(RwLock::new(SlotMap::with_key())),
            gong_watchers: Arc::new(RwLock::new(gong_watchers)),
            conditional_watchers: Arc::new(RwLock::new(SlotMap::with_key())),
            lifecycle_loops: Arc::new(RwLock::new(SlotMap::with_key())),
            lifecycle_triggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Runs the engine's main loop until a shutdown signal is received.
    ///
    /// This method will:
    /// 1. Spawn the `SystemClock` task.
    /// 2. Spawn the main dispatcher task that listens for ticks and fires events.
    /// 3. Wait for a Ctrl+C signal to initiate a graceful shutdown.
    pub async fn run(&self) -> anyhow::Result<()> {
        info!("HyperclockEngine starting up...");
        let (shutdown_tx, _) = broadcast::channel(1);

        let clock = SystemClock::new(self.config.resolution.clone(), self.tick_sender.clone());
        let clock_shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move { clock.run(clock_shutdown_rx).await });

        let dispatcher = self.clone();
        let dispatcher_shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move { dispatcher.dispatcher_loop(dispatcher_shutdown_rx).await });

        info!(
            "Engine running at {:?}. Press Ctrl+C to shut down.",
            self.config.resolution
        );
        tokio::signal::ctrl_c().await?;

        info!("Shutdown signal received. Broadcasting to all tasks...");
        if shutdown_tx.send(()).is_err() {
            error!("Failed to send shutdown signal. Some tasks may not terminate gracefully.");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.system_event_sender
            .send(SystemEvent::EngineShutdown)
            .ok();
        info!("HyperclockEngine has shut down.");
        Ok(())
    }

    #[doc(hidden)]
    async fn dispatcher_loop(self, mut shutdown_rx: broadcast::Receiver<()>) {
        let mut tick_rx = self.tick_sender.subscribe();
        let mut task_rx = self.task_event_sender.subscribe();
        self.system_event_sender
            .send(SystemEvent::EngineStarted {
                timestamp: tokio::time::Instant::now(),
            })
            .ok();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => break,
                Ok(tick) = tick_rx.recv() => {
                    trace!("Tick #{} received.", tick.tick_count);
                    self.process_tick_watchers(&tick).await;
                    for phase_config in self.config.phases.iter() {
                        let phase_event = PhaseEvent { phase: phase_config.id, tick: tick.clone() };
                        self.process_phase_watchers(&phase_event).await;
                        self.phase_sender.send(phase_event).ok();
                    }
                }
                Ok(task_event) = task_rx.recv() => {
                    if let TaskEvent::TaskFired { listener_id, .. } = task_event {
                         self.process_lifecycle_trigger(listener_id).await;
                    }
                }
            }
        }
    }

    #[doc(hidden)]
    async fn process_phase_watchers(&self, phase_event: &PhaseEvent) {
        let mut interval_watchers = self.interval_watchers.write().await;
        for (id, watcher) in interval_watchers.iter_mut() {
            if watcher.process_phase(phase_event.phase) {
                self.task_event_sender
                    .send(TaskEvent::TaskFired {
                        listener_id: id,
                        tick: phase_event.tick.clone(),
                    })
                    .ok();
            }
        }
    }

    #[doc(hidden)]
    async fn process_tick_watchers(&self, tick: &Arc<TickEvent>) {
        let mut conditional_watchers = self.conditional_watchers.write().await;
        let mut fired_one_shots = Vec::new();
        for (id, watcher) in conditional_watchers.iter_mut() {
            if watcher.check_and_fire() {
                self.conditional_event_sender
                    .send(ConditionalEvent {
                        condition_id: id,
                        timestamp: tick.timestamp,
                    })
                    .ok();
                if watcher.is_one_shot {
                    fired_one_shots.push(id);
                }
            }
        }
        for id in fired_one_shots {
            if conditional_watchers.remove(id).is_some() {
                self.system_event_sender
                    .send(SystemEvent::ListenerRemoved { id })
                    .ok();
            }
        }
        let mut gong_watchers = self.gong_watchers.write().await;
        for (_id, watcher) in gong_watchers.iter_mut() {
            watcher.process_tick(tick, &self.gong_event_sender);
        }
    }

    #[doc(hidden)]
    async fn process_lifecycle_trigger(&self, interval_listener_id: ListenerId) {
        let lifecycle_id = self
            .lifecycle_triggers
            .read()
            .await
            .get(&interval_listener_id)
            .copied();
        if let Some(id) = lifecycle_id {
            let mut loops = self.lifecycle_loops.write().await;
            let mut should_remove = false;
            if let Some(lifecycle) = loops.get_mut(id) {
                if lifecycle.advance(&self.automation_event_sender) {
                    should_remove = true;
                }
            }
            if should_remove {
                if let Some(removed_loop) = loops.remove(id) {
                    self.remove_interval_listener(removed_loop.listener_id).await;
                    self.lifecycle_triggers
                        .write()
                        .await
                        .remove(&removed_loop.listener_id);
                }
            }
        }
    }
}

// Public API implementation block.
impl HyperclockEngine {
    /// Registers a task to be executed at a regular interval.
    ///
    /// The task's interval timer will only advance during the specified `phase`.
    /// The provided `task_logic` closure will be executed each time the interval elapses.
    ///
    /// # Arguments
    /// * `phase_to_watch` - The `PhaseId` during which this interval is active.
    /// * `interval` - The `Duration` between task executions.
    /// * `task_logic` - A closure to execute when the interval fires.
    ///
    /// # Returns
    /// A `ListenerId` which can be used to later remove this watcher.
    pub async fn on_interval(
        &self,
        phase_to_watch: PhaseId,
        interval: Duration,
        task_logic: impl FnMut() + Send + Sync + 'static,
    ) -> ListenerId {
        let watcher = IntervalWatcher::new(phase_to_watch, interval, Box::new(task_logic));
        let mut watchers = self.interval_watchers.write().await;
        let id = watchers.insert(watcher);
        self.system_event_sender
            .send(SystemEvent::ListenerAdded { id })
            .ok();
        id
    }

    /// Registers a task to be executed whenever a given condition is met.
    ///
    /// The `condition` closure is checked on every tick of the engine. If it returns `true`,
    /// the `task_logic` closure is executed.
    ///
    /// # Arguments
    /// * `condition` - A closure that returns `true` when the task should fire.
    /// * `task_logic` - A closure to execute when the condition is met.
    /// * `is_one_shot` - If true, the watcher will be automatically removed after firing once.
    ///
    /// # Returns
    /// A `ListenerId` which can be used to later remove this watcher.
    pub async fn on_conditional(
        &self,
        condition: impl Fn() -> bool + Send + Sync + 'static,
        task_logic: impl FnMut() + Send + Sync + 'static,
        is_one_shot: bool,
    ) -> ListenerId {
        let watcher =
            ConditionalWatcher::new(Box::new(condition), Box::new(task_logic), is_one_shot);
        let mut watchers = self.conditional_watchers.write().await;
        let id = watchers.insert(watcher);
        self.system_event_sender
            .send(SystemEvent::ListenerAdded { id })
            .ok();
        id
    }

    /// Adds a new `LifecycleLoop` to the engine.
    ///
    /// This creates a complex automation that executes a sequence of steps,
    /// with each step advancing after the specified `interval`.
    ///
    /// # Returns
    /// A `TaskId` for the created lifecycle loop.
    pub async fn add_lifecycle_loop(
        &self,
        phase_to_watch: PhaseId,
        interval: Duration,
        steps: Vec<LifecycleStep>,
        repetition_policy: RepetitionPolicy,
    ) -> TaskId {
        // This interval watcher exists solely to drive the lifecycle loop.
        let interval_watcher_id = self
            .on_interval(phase_to_watch, interval, || {})
            .await;
        let mut loops = self.lifecycle_loops.write().await;
        let lifecycle_id = loops.insert_with_key(|key| {
            self.automation_event_sender
                .send(AutomationEvent::LifecycleStarted { id: key })
                .ok();
            LifecycleLoop::new(key, interval_watcher_id, steps, repetition_policy)
        });
        self.lifecycle_triggers
            .write()
            .await
            .insert(interval_watcher_id, lifecycle_id);
        lifecycle_id
    }

    /// Removes an interval listener from the engine.
    ///
    /// Returns `true` if the listener was found and removed.
    pub async fn remove_interval_listener(&self, id: ListenerId) -> bool {
        let was_removed = self.interval_watchers.write().await.remove(id).is_some();
        if was_removed {
            self.system_event_sender
                .send(SystemEvent::ListenerRemoved { id })
                .ok();
        }
        was_removed
    }

    /// Removes a conditional listener from the engine.
    ///
    /// Returns `true` if the listener was found and removed.
    pub async fn remove_conditional_listener(&self, id: ListenerId) -> bool {
        let was_removed = self.conditional_watchers.write().await.remove(id).is_some();
        if was_removed {
            self.system_event_sender
                .send(SystemEvent::ListenerRemoved { id })
                .ok();
        }
        was_removed
    }

    /// Removes a lifecycle loop from the engine.
    ///
    /// This also removes the internal interval watcher that drives the loop.
    /// Returns `true` if the loop was found and removed.
    pub async fn remove_lifecycle_loop(&self, id: TaskId) -> bool {
        if let Some(removed_loop) = self.lifecycle_loops.write().await.remove(id) {
            self.remove_interval_listener(removed_loop.listener_id)
                .await;
            self.lifecycle_triggers
                .write()
                .await
                .remove(&removed_loop.listener_id);
            true
        } else {
            false
        }
    }

    /// Subscribes to the `SystemEvent` stream.
    pub fn subscribe_system_events(&self) -> broadcast::Receiver<SystemEvent> {
        self.system_event_sender.subscribe()
    }

    /// Subscribes to the `PhaseEvent` stream.
    pub fn subscribe_phase_events(&self) -> broadcast::Receiver<PhaseEvent> {
        self.phase_sender.subscribe()
    }

    /// Subscribes to the `GongEvent` stream.
    pub fn subscribe_gong_events(&self) -> broadcast::Receiver<GongEvent> {
        self.gong_event_sender.subscribe()
    }

    /// Subscribes to the `TaskEvent` stream.
    pub fn subscribe_task_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.task_event_sender.subscribe()
    }

    /// Subscribes to the `AutomationEvent` stream.
    pub fn subscribe_automation_events(&self) -> broadcast::Receiver<AutomationEvent> {
        self.automation_event_sender.subscribe()
    }

    /// Subscribes to the `ConditionalEvent` stream.
    pub fn subscribe_conditional_events(&self) -> broadcast::Receiver<ConditionalEvent> {
        self.conditional_event_sender.subscribe()
    }

    /// Subscribes to the `UserEvent` stream.
    pub fn subscribe_user_events(&self) -> broadcast::Receiver<UserEvent> {
        self.user_event_sender.subscribe()
    }

    /// Broadcasts a custom `UserEvent` to all subscribers.
    pub fn broadcast_user_event(&self, event: UserEvent) {
        self.user_event_sender.send(event).ok();
    }
}
