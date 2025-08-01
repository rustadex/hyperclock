//! A robust, concurrent multi-clock engine for temporal simulations.
//!
//! This example builds on the simple multi-clock concept by adding:
//! 1.  **Shared State:** A central `SimulationState` wrapped in an `Arc<RwLock<...>>`
//!     allows all clocks to safely read from and write to each other's data buffers.
//! 2.  **Graceful Shutdown:** The engine listens for a Ctrl+C signal and uses a
//!     broadcast channel to tell all running clock tasks to terminate cleanly.
//! 3.  **Centralized Control:** An `Engine` struct encapsulates all the components,
//!     making the system a single, reusable unit.
//! 4.  **Asynchronous Callbacks:** The clock's run loop now accepts an async callback,
//!     allowing for `.await`-ing operations like acquiring locks within a tick.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval_at, Instant};

/// Holds the shared data buffers that all clocks can access.
#[derive(Debug, Default)]
struct SimulationState {
    lookahead_buffer: i32,
    prime_buffer: i32,
    memory_buffer: i32,
}

/// A configurable temporal clock that runs as an independent asynchronous task.
struct TemporalClock {
    name: &'static str,
    interval: Duration,
    offset: Duration,
}

impl TemporalClock {
    fn new(name: &'static str, interval: Duration, offset: Duration) -> Self {
        Self { name, interval, offset }
    }

    /// Spawns the clock's main loop on a new Tokio task.
    ///
    /// The loop will:
    /// - Tick at the specified `interval` after an initial `offset`.
    /// - Execute the provided `async` callback on each tick.
    /// - Listen for a shutdown signal and exit gracefully.
    async fn spawn_task<F, Fut>(&self, mut callback: F, mut shutdown_rx: broadcast::Receiver<()>)
    where
        F: FnMut(&'static str) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let name = self.name;
        let start = Instant::now() + self.offset;
        let mut ticker = interval_at(start, self.interval);

        // Spawn the core logic in its own task.
        tokio::spawn(async move {
            println!("[{}] Clock task started.", name);
            loop {
                tokio::select! {
                    // Wait for the next tick.
                    _ = ticker.tick() => {
                        // Execute the user-defined async logic for this tick.
                        callback(name).await;
                    }
                    // Wait for a shutdown signal from the engine.
                    _ = shutdown_rx.recv() => {
                        println!("[{}] Shutting down...", name);
                        break; // Exit the loop.
                    }
                }
            }
        });
    }
}

/// The main engine that orchestrates the simulation.
struct Engine {
    state: Arc<RwLock<SimulationState>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Engine {
    /// Creates a new Engine with initialized state and shutdown channel.
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            state: Arc::new(RwLock::new(SimulationState::default())),
            shutdown_tx,
        }
    }

    /// Runs the entire simulation.
    ///
    /// This method will:
    /// 1. Set up and spawn the individual clock tasks.
    /// 2. Wait for a Ctrl+C signal to begin shutdown.
    /// 3. Broadcast the shutdown signal to all tasks.
    pub async fn run(&self) {
        println!("Engine starting... Press Ctrl+C to exit.");

        let period = Duration::from_secs(2);

        // Define our clocks.
        let prime_clock = TemporalClock::new("Prime", period, Duration::from_secs(0));
        let lookahead_clock = TemporalClock::new("Lookahead", period, period);
        let memory_clock = TemporalClock::new("Memory", period, Duration::from_secs(0));

        // Spawn the Lookahead Clock Task
        let lookahead_state = self.state.clone();
        lookahead_clock.spawn_task(move |_name| {
            let state_clone = lookahead_state.clone();
            async move {
                let mut state = state_clone.write().await;
                state.lookahead_buffer += 1; // Generate the "future" value.
                println!("[Lookahead] Tick. New buffer value: {}", state.lookahead_buffer);
            }
        }, self.shutdown_tx.subscribe()).await;

        // Spawn the Prime Clock Task
        let prime_state = self.state.clone();
        prime_clock.spawn_task(move |_name| {
            let state_clone = prime_state.clone();
            async move {
                let mut state = state_clone.write().await;
                // Prime reads the value that lookahead just generated.
                let future_val = state.lookahead_buffer;
                println!("[Prime] Tick. Read lookahead value: {}. Applying logic...", future_val);
                state.prime_buffer = future_val; // For simplicity, we just copy it.
            }
        }, self.shutdown_tx.subscribe()).await;

        // Spawn the Memory Clock Task
        let memory_state = self.state.clone();
        memory_clock.spawn_task(move |_name| {
            let state_clone = memory_state.clone();
            async move {
                let mut state = state_clone.write().await;
                // Memory reads the value that prime just set.
                let prime_val = state.prime_buffer;
                println!("[Memory] Tick. Read prime value: {}. Storing in memory...", prime_val);
                state.memory_buffer = prime_val;
            }
        }, self.shutdown_tx.subscribe()).await;


        // Wait for the shutdown signal (Ctrl+C).
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Error waiting for shutdown signal: {}", e);
        }

        println!("\nShutdown signal received. Broadcasting to all tasks...");
        // Sending a signal on the broadcast channel will cause all live `shutdown_rx.recv()`
        // calls to complete, breaking their loops. A `_` is used for the result because
        // it can error if there are no receivers, which is fine.
        let _ = self.shutdown_tx.send(());

        // Give tasks a moment to print their shutdown messages.
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("Engine stopped.");
    }
}


#[tokio::main]
async fn main() {
    let engine = Engine::new();
    engine.run().await;
}
