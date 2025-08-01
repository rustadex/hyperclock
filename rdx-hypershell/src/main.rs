use anyhow::Result;
use colored::Colorize;
use hyperclock::{ENGINE_NAME, VERSION as LIB_VERSION};
use hyperclock::prelude::*;
use std::env;
use std::time::Duration;
use tracing::info;

const SHELL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {


    let config = HyperclockConfig::default();
    let engine = HyperclockEngine::new(config);
    let engine_handle = engine.clone();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    if env::var("QUIET_MODE").is_err() {
        // The `include_str!` macro reads the file at COMPILE time and embeds
        // the text directly into the binary. This is very efficient.
        // It assumes `logo.log` is in the root of the `rdx-hypershell` crate.
        const LOGO_TEXT: &str = include_str!("../logo.log");
        println!("{}", LOGO_TEXT.blue());

        // Dynamically create the version string
        let version_string = format!(
            "          Shell   v{:<8} Library   v{:<8}",
            SHELL_VERSION, LIB_VERSION
        );        

        println!("{}", "------------------------------------".dimmed());

        // --- NEW: ADD THE LICENSE BLURB ---
        let license_blurb = "
        This software is provided 'as is', without warranty of any kind.
        Distributed under the MIT OR Apache-2.0 license. Use at your own risk.
        ";

        println!("{}", version_string);
        println!("{}", license_blurb.dimmed());

        println!("{}", "------------------------------------".dimmed());


    }
    

    let mut system_rx = engine_handle.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            println!("\n<-- [SYSTEM EVENT] {:?}\n>> ", event);
        }
    });

    info!("Spawning {} in the background...", ENGINE_NAME);
    tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            eprintln!("\nEngine stopped with an error: {}", e);
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // The shell's state is declared here, before the loop.
    // It is `mut` because we `push` new IDs into it.
    let mut active_listeners: Vec<ListenerId> = Vec::new();

    let mut rl = rustyline::DefaultEditor::new()?;
    println!("{} is running. Type 'help' for commands or 'exit' to quit.", ENGINE_NAME);

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let args = line.trim().split_whitespace().collect::<Vec<_>>();

                if let Some(command) = args.get(0) {
                    match *command {
                        "add" => {
                            if let Some(&"interval") = args.get(1) {
                                let listener_id = engine_handle.on_interval(
                                    PhaseId(0),
                                    Duration::from_secs(5),
                                    || println!("<-- [INTERVAL TASK] A 5-second interval fired!"),
                                ).await;
                                let handle = active_listeners.len();
                                active_listeners.push(listener_id); // This line requires `mut`.
                                println!("--> Added interval listener with handle: #{}", handle);
                            } else {
                                println!("Unknown 'add' command. Try 'add interval'.");
                            }
                        }
                        "remove" => {
                            if let Some(&"interval") = args.get(1) {
                                if let Some(handle_str) = args.get(2) {
                                    if let Ok(handle) = handle_str.parse::<usize>() {
                                        if let Some(id_to_remove) = active_listeners.get(handle).cloned() {
                                            if engine_handle.remove_interval_listener(id_to_remove).await {
                                                println!("--> Listener successfully removed.");
                                            } else {
                                                println!("--> Error: Listener not found in engine.");
                                            }
                                        } else {
                                             println!("Error: Invalid handle #{}. Use 'list' to see active listeners.", handle);
                                        }
                                    } else {
                                        println!("Error: Handle must be a number (e.g., '0', '1').");
                                    }
                                } else {
                                    println!("Usage: remove interval <HANDLE>");
                                }
                            } else {
                                println!("Unknown 'remove' command. Try 'remove interval'.");
                            }
                        }
                        "list" => {
                             println!("Active Listeners:");
                             for (handle, id) in active_listeners.iter().enumerate() {
                                 println!("  Handle #{}: {:?}", handle, id);
                             }
                        }
                        "help" => {
                            println!("Available commands:");
                            println!("  add interval          - Adds a 5-second interval watcher.");
                            println!("  list                  - Shows active listeners and their handles.");
                            println!("  remove interval <H>   - Removes a watcher by its handle (e.g., remove interval 0).");
                            println!("  exit                  - Quits the shell.");
                        }
                        "exit" => break,
                        "" => {}
                        _ => println!("Unknown command: '{}'. Type 'help'.", line),
                    }
                }
            }
            Err(_) => {
                println!("Exiting hypershell...");
                break;
            }
        }
    }

    Ok(())
}
