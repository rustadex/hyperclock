use anyhow::Result;
use colored::Colorize;
use hyperclock::{ENGINE_NAME, VERSION as LIB_VERSION};
use hyperclock::prelude::*;
use rustyline::highlight::Highlighter;
use rustyline::Editor;
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
// NEW IMPORTS for the shared flag
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

const SHELL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A custom helper struct for rustyline that enables syntax highlighting.
#[derive(Completer, Helper, Hinter, Validator)]
struct MyHighlighter;

impl Highlighter for MyHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if let Some((command, rest)) = line.split_once(' ') {
            let colored_command = command.yellow().bold();
            let colored_rest = rest.yellow();
            Cow::Owned(format!("{} {}", colored_command, colored_rest))
        } else {
            Cow::Owned(line.yellow().bold().to_string())
        }
    }
    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

/// Reads the logo file, parses color hints, and prints the banner.
// fn print_banner() {
//     if env::var("QUIET_MODE").is_ok() {
//         return;
//     }
//     const LOGO_TEXT: &str = include_str!("../logo.log");
//     for line in LOGO_TEXT.lines() {
//         if line.starts_with("B|") {
//             println!("{}", &line[2..].blue());
//         } else if line.starts_with("C|") {
//             println!("{}", &line[2..].cyan());
//         } else if line.starts_with("D|") {
//             println!("{}", &line[2..].dimmed());
//         } else {
//             println!("{}", line);
//         }
//     }
//     let version_string = format!(
//         "          Shell   v{:<8} Library   v{:<8}",
//         SHELL_VERSION, LIB_VERSION
//     );
//     println!("{}", version_string);
//     let license_blurb = "
// This software is provided 'as is', without warranty of any kind.
// Distributed under the MIT OR Apache-2.0 license. Use at your own risk.
// ";
//     println!("{}", license_blurb.dimmed());
// }

fn print_banner() {
    if env::var("QUIET_MODE").is_ok() {
        return;
    }
    // The `include_str!` macro reads the file at COMPILE time and embeds
    // the text directly into the binary. This is very efficient.
    // It assumes `logo.log` is in the root of the `rdx-hypershell` crate.
    const LOGO_TEXT: &str = include_str!("../logo.log");
    println!("{}", LOGO_TEXT.cyan());

    // Dynamically create the version string
    let version_string = format!(
        "          Shell   v{:<8} Library   v{:<8}",
        SHELL_VERSION, LIB_VERSION
    );        

    println!("{}", "-----------------------------------------------------------------------------------------------".dimmed());

    // --- NEW: ADD THE LICENSE BLURB ---
    let license_blurb = "
    This software is provided 'as is', without warranty of any kind.
    Distributed under the MIT OR Apache-2.0 license. Use at your own risk.
    ";

    println!("{}", version_string);
    println!("{}", license_blurb.dimmed());

    println!("{}", "-----------------------------------------------------------------------------------------------".dimmed());

}

/// Spawns several tasks, each subscribing to a different event stream from the engine.
fn spawn_event_listeners(engine: &HyperclockEngine, is_listening_to_ticks: Arc<AtomicBool>) {
    // System Event Listener
    let mut system_rx = engine.subscribe_system_events();
    tokio::spawn(async move {
        while let Ok(event) = system_rx.recv().await {
            println!("\n<-- [SYSTEM EVENT] {:?}\n>> ", event);
        }
    });

    // Tick Listener (controlled by the shared flag)
    let mut tick_rx = engine.subscribe_tick_events();
    tokio::spawn(async move {
        while let Ok(event) = tick_rx.recv().await {
            if is_listening_to_ticks.load(Ordering::Relaxed) {
                if event.tick_count % 5 == 0 {
                    println!("<-- [RAW TICK] Tick #{}", event.tick_count);
                }
            }
        }
        // while let Ok(event) = tick_rx.recv().await {
        //     // Check the on/off flag first.
        //     if is_listening_to_ticks.load(Ordering::Relaxed) {
        //         // Now, check if one second has passed since the last print.
        //         if last_print_time.elapsed() >= Duration::from_secs(1) {
        //             println!("<-- [TICK SUMMARY] Latest tick: #{}", event.tick_count);
        //             // Reset the timer for the next second.
        //             last_print_time = Instant::now();
        //         }
        //     }
        // }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();


    // let config = HyperclockConfig {
    //     resolution: ClockResolution::Ultra, // <-- IT JUST WORKS!
    //     ..Default::default()
    // };

    let config = HyperclockConfig::default();
    let engine = HyperclockEngine::new(config);
    let engine_handle = engine.clone();

    // Create the shared flag for the tick listener.
    let is_listening_to_ticks = Arc::new(AtomicBool::new(false));

    // Pass a clone of the flag to the event listeners.
    spawn_event_listeners(&engine_handle, is_listening_to_ticks.clone());

    info!("Spawning {} in the background...", ENGINE_NAME.cyan());
    tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            eprintln!("\nEngine stopped with an error: {}", e);
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // The shell's state management variables.
    let mut active_listeners: HashMap<usize, ListenerId> = HashMap::new();
    let mut next_handle: usize = 0;

    let mut rl = Editor::new()?;
    let helper = MyHighlighter {};
    rl.set_helper(Some(helper));

    println!("{} is running. Type 'help' for commands or 'exit' to quit.", ENGINE_NAME.cyan());

    loop {
        let prompt = format!("{}", ">> ".cyan().bold());
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let args = line.trim().split_whitespace().collect::<Vec<_>>();

                if let Some(command) = args.get(0) {
                    match *command {
                        "add" => {
                            if let Some(&"interval") = args.get(1) {
                                if let Some(seconds_str) = args.get(2) {
                                    if let Ok(seconds) = seconds_str.parse::<u64>() {
                                        let listener_id = engine_handle.on_interval(
                                            PhaseId(0),
                                            Duration::from_secs(seconds),
                                            move || println!("<-- [INTERVAL TASK] A {}-second interval fired!", seconds),
                                        ).await;
                                        let handle = next_handle;
                                        active_listeners.insert(handle, listener_id);
                                        next_handle += 1;
                                        println!("--> Added {}-second interval listener with handle: #{}", seconds, handle);
                                    } else {
                                        println!("Error: '{}' is not a valid number of seconds.", seconds_str);
                                    }
                                } else {
                                    println!("Usage: add interval <SECONDS>");
                                }
                            } else {
                                println!("Unknown 'add' command. Try 'add interval'.");
                            }
                        }
                        "remove" => {
                           if let Some(&"interval") = args.get(1) {
                                if let Some(handle_str) = args.get(2) {
                                    if let Ok(handle) = handle_str.parse::<usize>() {
                                        if let Some(id_to_remove) = active_listeners.get(&handle).cloned() {
                                            if engine_handle.remove_interval_listener(id_to_remove).await {
                                                println!("--> Listener successfully removed.");
                                                active_listeners.remove(&handle);
                                            } else {
                                                println!("--> Error: Listener not found in engine.");
                                                active_listeners.remove(&handle);
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
                             for (handle, id) in &active_listeners {
                                 println!("  Handle #{}: {:?}", handle, id);
                             }
                        }
                        "start" => {
                            if let Some(&"ticks") = args.get(1) {
                                is_listening_to_ticks.store(true, Ordering::Relaxed);
                                println!("--> Started listening to raw tick stream.");
                            } else {
                                println!("Unknown 'start' command. Try 'start ticks'.");
                            }
                        }
                        "stop" => {
                            if let Some(&"ticks") = args.get(1) {
                                is_listening_to_ticks.store(false, Ordering::Relaxed);
                                println!("--> Stopped listening to raw tick stream.");
                            } else {
                                println!("Unknown 'stop' command. Try 'stop ticks'.");
                            }
                        }
                        "help" => {
                            println!("Available commands:");
                            println!("  add interval <S>      - Adds an S-second interval watcher.");
                            println!("  list                  - Shows active listeners and their handles.");
                            println!("  remove interval <H>   - Removes a watcher by its handle.");
                            println!("  start ticks           - Begins printing the raw tick stream.");
                            println!("  stop ticks            - Stops printing the raw tick stream.");
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
