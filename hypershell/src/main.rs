use anyhow::Result;
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
    // In a real app, you might load this from a file.
    let config = HyperclockConfig::default();
    let engine = HyperclockEngine::new(config);

    // 3. Clone the engine handle to use for sending commands.
    // The original `engine` will be moved into the background task.
    let engine_handle = engine.clone();

    // 4. Spawn a task to listen to system events for feedback.
    let mut system_rx = engine_handle.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            // Use println! here because this is user-facing feedback, not just a log.
            println!("\n<-- [SYSTEM EVENT] {:?}\n>> ", event);
        }
    });

    // 5. Spawn the engine to run in the background.
    // This is the most important step! We `move` the engine into the task.
    info!("Spawning Hyperclock engine in the background...");
    tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            eprintln!("Engine failed with error: {}", e);
        }
    });
    
    // Give the engine a moment to start up.
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

                // 7. Parse the user's command and call the engine's public API.
                // We use the `engine_handle` to interact with the background engine.
                if let Some(command) = args.get(0) {
                    match *command {
                        "add" => {
                            if let Some(&"interval") = args.get(1) {
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
                            // TODO: Add logic to parse a ListenerId and call remove_interval_listener
                            println!("Remove command not yet implemented.");
                        }
                        "help" => {
                            println!("Available commands:");
                            println!("  add interval - Adds a 5-second interval watcher.");
                            println!("  exit         - Quits the shell.");
                        }
                        "exit" => break,
                        "" => {} // Ignore empty input
                        _ => println!("Unknown command: '{}'. Type 'help'.", line),
                    }
                }
            }
            Err(_) => {
                // This handles Ctrl+C or Ctrl+D in the prompt.
                println!("Exiting hypershell...");
                break;
            }
        }
    }

    // The engine will continue running in the background until the main process exits.
    // For a clean shutdown, we would need to add a command to tell the engine to stop.
    Ok(())
}
