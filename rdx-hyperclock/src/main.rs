use anyhow::Result;
// CORRECTED: Import items from the `hyperclock` library crate by name.
use hyperclock::components::task::{LifecycleStep, RepetitionPolicy};
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
        resolution: ClockResolution::Medium,
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
        // CORRECTED: Also needs to refer to the library crate by name.
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

    // 6. Run the engine.
    engine.run().await?;

    Ok(())
}

/// Spawns several tasks, each subscribing to a different event stream from the engine.
fn spawn_event_listeners(engine: &HyperclockEngine) {
    let mut system_rx = engine.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            info!("[SYSTEM] => {:?}", event);
        }
    });

    let mut task_rx = engine.subscribe_task_events();
    tokio::spawn(async move {
        while let Ok(event) = task_rx.recv().await {
            info!("[TASK] => {:?}", event);
        }
    });

    let mut automation_rx = engine.subscribe_automation_events();
    tokio::spawn(async move {
        while let Ok(event) = automation_rx.recv().await {
            info!("[AUTOMATION] => {:?}", event);
        }
    });

    let mut conditional_rx = engine.subscribe_conditional_events();
    tokio::spawn(async move {
        while let Ok(event) = conditional_rx.recv().await {
            info!("[CONDITIONAL] => Condition met: {:?}", event.condition_id);
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
            info!("[INTERVAL TASK] Counter is now: {}", current);
        })
        .await;

    // --- Register a repeating conditional watcher ---
    let counter_clone = shared_counter.clone();
    let _repeating_cond_id = engine
        .on_conditional(
            move || counter_clone.load(Ordering::Relaxed) >= 3,
            || info!("[REPEATING CONDITIONAL] Condition 'counter >= 3' is TRUE."),
            false,
        )
        .await;

    // --- Register a one-shot conditional watcher ---
    let counter_clone = shared_counter.clone();
    let _one_shot_cond_id = engine
        .on_conditional(
            move || counter_clone.load(Ordering::Relaxed) == 5,
            || info!("[ONE-SHOT CONDITIONAL] Fired! This task will now be removed."),
            true,
        )
        .await;

    // --- Register a Lifecycle Loop ---
    let steps: Vec<LifecycleStep> = vec![
        Box::new(|| info!("[LIFECYCLE] => Step 1: Initializing...")),
        Box::new(|| info!("[LIFECYCLE] => Step 2: Processing...")),
        Box::new(|| info!("[LIFECYCLE] => Step 3: Finalizing cycle.")),
    ];

    let _lifecycle_id = engine
        .add_lifecycle_loop(
            PhaseId(0),
            Duration::from_secs(1),
            steps,
            RepetitionPolicy::RunNTimes(2),
        )
        .await;
}
