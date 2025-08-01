use anyhow::Result;
use hyperclock::config::{ClockResolution, PhaseConfig};
use hyperclock::prelude::*;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging for the shell application.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // 2. Create a default configuration for the engine.
    // In a real app, you would load this from a `hypershell.toml` file.
    let config = HyperclockConfig {
        resolution: ClockResolution::Medium,
        phases: vec![PhaseConfig {
            id: PhaseId(0),
            label: "default".to_string(),
        }],
        ..Default::default()
    };
    let engine = HyperclockEngine::new(config);

    // 3. Clone the engine handle. This handle is our "client" that we will use
    // to send commands to the running engine.
    let engine_handle = engine.clone();

    // 4. Spawn a task to listen to system events for user feedback.
    let mut system_rx = engine_handle.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            // Using println! here because this is direct user feedback, not just a log.
            println!("\n<-- [SYSTEM EVENT] {:?}\n>> ", event);
        }
    });

    // 5. Spawn the engine to run in the background. We `move` the original
    // `engine` into this task, where it will live for the duration of the program.
    info!("Spawning Hyperclock engine in the background...");
    tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            eprintln!("\nEngine stopped with an error: {}", e);
        }
    });

    // Give the engine a moment to start up before showing the prompt.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 6. Start the interactive command loop (REPL).
    let mut rl = rustyline::DefaultEditor::new()?;
    println!("--- hypershell ---");
    println!("Hyperclock is running. Type 'help' for commands or 'exit' to quit.");

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let args = line.trim().split_whitespace().collect::<Vec<_>>();

                // 7. Parse the user's command and call the engine's public API
                // using our `engine_handle`.
                if let Some(command) = args.get(0) {
                    match *command {
                        "add" => {
                            if let Some(&"interval") = args.get(1) {
                                // We call the async API method on our handle.
                                let listener_id = engine_handle.on_interval(
                                    PhaseId(0),
                                    Duration::from_secs(5),
                                    || println!("<-- [INTERVAL TASK] A 5-second interval fired!"),
                                ).await;
                                println!("--> Added interval listener with ID: {:?}", listener_id);
                            } else {
                                println!("Unknown 'add' command. Try 'add interval'.");
                            }
                        }
                        "remove" => {
                            println!("Remove command not yet implemented.");
                        }
                        "help" => {
                            println!("Available commands:");
                            println!("  add interval - Adds a 5-second interval watcher on phase 0.");
                            println!("  exit         - Quits the shell.");
                        }
                        "exit" => break,
                        "" => {} // Ignore empty input
                        _ => println!("Unknown command: '{}'. Type 'help'.", line),
                    }
                }
            }
            Err(_) => { // This handles Ctrl+C or Ctrl+D in the prompt.
                println!("Exiting hypershell...");
                break;
            }
        }
    }

    // When main exits, the background engine task will be dropped and shut down.
    Ok(())
}
