use anyhow::Result;
use hyperclock::components::task::RepetitionPolicy;
use hyperclock::config::{ClockResolution, Holiday, PhaseConfig};
use hyperclock::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize structured logging.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // 2. Create a custom configuration for the engine.
    let config = HyperclockConfig {
        resolution: ClockResolution::Medium, // ~30 ticks per second
        phases: vec![
            PhaseConfig {
                id: PhaseId(0),
                label: "logic".to_string(),
            },
            PhaseConfig {
                id: PhaseId(1),
                label: "rendering".to_string(),
            },
        ],
        gong_config: hyperclock::config::GongConfig {
            holidays: vec![Holiday {
                name: "New Year's Day".to_string(),
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            }],
            ..Default::default()
        },
    };

    // 3. Create the HyperclockEngine instance.
    let engine = HyperclockEngine::new(config);

    // 4. Spawn concurrent tasks to listen to different event streams.
    spawn_event_listeners(&engine);

    // 5. Register watchers and loops to test the engine's core logic.
    register_test_components(&engine).await;

    // 6. Run the engine. This is a blocking call that will only return
    //    when the application receives a shutdown signal (Ctrl+C).
    engine.run().await?;

    Ok(())
}

/// Spawns several tasks, each subscribing to a different event stream from the engine.
fn spawn_event_listeners(engine: &HyperclockEngine) {
    // --- System Event Listener ---
    let mut system_rx = engine.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            info!("[SYSTEM] => {:?}", event);
        }
    });

    // --- Phase Event Listener (logs every 30th tick to avoid spam) ---
    let mut phase_rx = engine.subscribe_phase_events();
    tokio::spawn(async move {
        while let Ok(event) = phase_rx.recv().await {
            if event.tick.tick_count % 30 == 0 {
                info!(
                    "[PHASE] => Phase #{} on Tick #{}",
                    (event.phase).0,
                    event.tick.tick_count
                );
            }
        }
    });

    // --- Gong Event Listener ---
    let mut gong_rx = engine.subscribe_gong_events();
    tokio::spawn(async move {
        while let Ok(event) = gong_rx.recv().await {
            info!("[GONG] => {:?}", event);
        }
    });

    // --- Conditional Event Listener ---
    let mut conditional_rx = engine.subscribe_conditional_events();
    tokio::spawn(async move {
        while let Ok(event) = conditional_rx.recv().await {
            info!("[CONDITIONAL] => Condition met: {:?}", event.condition_id);
        }
    });

    // --- NEW: Automation Event Listener ---
    let mut automation_rx = engine.subscribe_automation_events();
    tokio::spawn(async move {
        while let Ok(event) = automation_rx.recv().await {
            info!("[AUTOMATION] => {:?}", event);
        }
    });
}

/// Registers test components with the engine to demonstrate functionality.
async fn register_test_components(engine: &HyperclockEngine) {
    let shared_counter = Arc::new(AtomicU32::new(0));

    // --- Register a 2-second interval watcher ---
    let counter_clone = shared_counter.clone();
    let _interval_id = engine
        .on_interval(PhaseId(0), Duration::from_secs(2), move || {
            let current = counter_clone.fetch_add(1, Ordering::Relaxed) + 1;
            info!("[INTERVAL TASK] => Counter is now: {}", current);
        })
        .await;

    // --- Register a repeating conditional watcher ---
    let counter_clone = shared_counter.clone();
    let _repeating_cond_id = engine
        .on_conditional(
            move || counter_clone.load(Ordering::Relaxed) >= 3,
            || info!("[REPEATING CONDITIONAL TASK] => Condition 'counter >= 3' is TRUE."),
            false, // `is_one_shot = false`
        )
        .await;

    // --- Register a one-shot conditional watcher ---
    let counter_clone = shared_counter.clone();
    let _one_shot_cond_id = engine
        .on_conditional(
            move || counter_clone.load(Ordering::Relaxed) == 5,
            || info!("[ONE-SHOT CONDITIONAL TASK] => Fired! This task will be removed."),
            true, // `is_one_shot = true`
        )
        .await;

    // --- NEW: Register a Lifecycle Loop ---
    // This loop will have 3 steps, advance every 1 second, and will repeat twice before completing.
    let _lifecycle_id = engine
        .add_lifecycle_loop(
            PhaseId(0), // The interval advances during phase 0
            Duration::from_secs(1),
            vec![
                Box::new(|| info!("[LIFECYCLE] => Step 1: Initializing...")),
                Box::new(|| info!("[LIFECYCLE] => Step 2: Processing...")),
                Box::new(|| info!("[LIFECYCLE] => Step 3: Fun cycle.")),
                Box::new(|| info!("[LIFECYCLE] => Step 4: Finalizing cycle.")),
            ],
            RepetitionPolicy::RunNTimes(2),
        )
        .await;
}
