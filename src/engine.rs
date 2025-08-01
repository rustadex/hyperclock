//! The core engine that orchestrates the entire Hyperclock system.

use crate::common::{ListenerId, PhaseId, TaskId};
use crate::components::watcher::{ConditionalWatcher, GongWatcher, IntervalWatcher};
use crate::config::HyperclockConfig;
use crate::events::{
    AutomationEvent, ConditionalEvent, GongEvent, PhaseEvent, SystemEvent, TaskEvent, UserEvent,
};
use crate::time::{SystemClock, TickEvent};
use slotmap::{SlotMap, Key};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, trace};

/// The main Hyperclock engine.
///
/// This struct is the central point of control. It holds the system's configuration,
/// manages all active listeners and tasks, and drives the event loop.
#[derive(Clone)]
pub struct HyperclockEngine {
    config: Arc<HyperclockConfig>,

    // --- Senders for each public event category ---
    tick_sender: broadcast::Sender<Arc<TickEvent>>,
    phase_sender: broadcast::Sender<PhaseEvent>,
    system_event_sender: broadcast::Sender<SystemEvent>,
    gong_event_sender: broadcast::Sender<GongEvent>,
    task_event_sender: broadcast::Sender<TaskEvent>,
    automation_event_sender: broadcast::Sender<AutomationEvent>,
    conditional_event_sender: broadcast::Sender<ConditionalEvent>,
    user_event_sender: broadcast::Sender<UserEvent>,

    // --- Thread-Safe Storage for active components ---
    interval_watchers: Arc<RwLock<SlotMap<ListenerId, IntervalWatcher>>>,
    gong_watchers: Arc<RwLock<SlotMap<ListenerId, GongWatcher>>>,
    conditional_watchers: Arc<RwLock<SlotMap<ListenerId, ConditionalWatcher>>>,
}

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

        Self {
            config: Arc::new(config),
            tick_sender,
            phase_sender,
            system_event_sender,
            gong_event_sender,
            task_event_sender,
            automation_event_sender,
            conditional_event_sender,
            user_event_sender,
            interval_watchers: Arc::new(RwLock::new(SlotMap::with_key())),
            gong_watchers: Arc::new(RwLock::new(SlotMap::with_key())),
            conditional_watchers: Arc::new(RwLock::new(SlotMap::with_key())),
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
        tokio::spawn(clock.run(shutdown_tx.subscribe()));

        let dispatcher = self.clone();
        tokio::spawn(dispatcher.dispatcher_loop(shutdown_tx.subscribe()));

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
        self.system_event_sender.send(SystemEvent::EngineShutdown).ok();
        info!("HyperclockEngine has shut down.");
        Ok(())
    }

    /// The core event processing loop of the engine.
    #[doc(hidden)]
    async fn dispatcher_loop(&self, mut shutdown_rx: broadcast::Receiver<()>) {
        let mut phase_rx = self.phase_sender.subscribe();
        self.system_event_sender.send(SystemEvent::EngineStarted {
            timestamp: tokio::time::Instant::now(),
        }).ok();

        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => break,
                Ok(phase_event) = phase_rx.recv() => {
                    trace!("Phase #{} on Tick #{} received. Processing watchers...", (phase_event.phase).0, phase_event.tick.tick_count);
                    self.process_all_watchers(&phase_event).await;
                }
            }
        }
    }

    /// Processes all active watchers for a given phase event.
    async fn process_all_watchers(&self, phase_event: &PhaseEvent) {
        // --- Process Interval Watchers ---
        // We need a write lock because the watcher's logic is `FnMut`.
        let mut interval_watchers = self.interval_watchers.write().await;
        for (id, watcher) in interval_watchers.iter_mut() {
            watcher.process_phase(phase_event.phase);
        }

        // --- Process Conditional Watchers ---
        // This is more complex as one-shot watchers need to be removed.
        let mut conditional_watchers = self.conditional_watchers.write().await;
        let mut fired_one_shots = Vec::new();
        for (id, watcher) in conditional_watchers.iter_mut() {
            if watcher.check_and_fire() {
                self.conditional_event_sender.send(ConditionalEvent {
                    condition_id: id,
                    timestamp: tokio::time::Instant::now(),
                }).ok();
                if watcher.is_one_shot {
                    fired_one_shots.push(id);
                }
            }
        }
        // Remove any one-shot watchers that fired.
        for id in fired_one_shots {
            conditional_watchers.remove(id);
            self.system_event_sender.send(SystemEvent::ListenerRemoved { id }).ok();
        }

        // --- Process Gong Watchers (only on the first phase of a tick) ---
        if phase_event.phase.0 == 0 {
            let mut gong_watchers = self.gong_watchers.write().await;
            for (_id, watcher) in gong_watchers.iter_mut() {
                watcher.process_tick(&phase_event.tick, &self.gong_event_sender);
            }
        }
    }

    // --- Public API for Adding Listeners ---

    /// Registers a task to be executed at a regular interval.
    ///
    /// The task's interval timer will only advance during the specified `phase`.
    pub async fn on_interval(
        &self,
        phase_to_watch: PhaseId,
        interval: Duration,
        task_logic: impl FnMut() + Send + Sync + 'static,
    ) -> ListenerId {
        let task_id = TaskId(ListenerId::null().data()); // Placeholder until tasks are separate
        let watcher = IntervalWatcher::new(task_id, phase_to_watch, interval, Box::new(task_logic));
        let mut watchers = self.interval_watchers.write().await;
        let id = watchers.insert(watcher);
        self.system_event_sender.send(SystemEvent::ListenerAdded { id }).ok();
        id
    }

    // --- Public API for Subscribing to Events ---

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
    
    // ... and so on for other event types ...
}
