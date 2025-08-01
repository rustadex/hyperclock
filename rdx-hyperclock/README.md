
# 🕰️ Hyperclock


<p align="center">
  <img src="https://raw.githubusercontent.com/rustadex/hyperclock/main/.github/assets/logo.png" alt="placerholder" width="80%">
</p>

[![Crates.io Version](https://img.shields.io/crates/v/rdx-hyperclock.svg)](https://crates.io/crates/rdx-hyperclock)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/rdx-hyperclock.svg)](https://github.com/rustadex/rdx-hyperclock/blob/main/LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/msrv-1.70.0-blue.svg)](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html)

A high-performance, event-driven, phased time engine for Rust, designed for simulations, game development, and complex, time-sensitive applications.



## What is Hyperclock?

Hyperclock is a library for building complex, time-sensitive, and event-driven applications in Rust. It provides a powerful "phasetime" engine that acts as the central nervous system for your project, allowing you to schedule logic, react to events, and manage complex automations with a clean, decoupled architecture.

It's designed for applications that need to manage a lot of concurrent logic that happens over time, such as:
*   Game engines (especially for managing game loops and AI ticks)
*   Simulations and modeling systems
*   Interactive media and robotics
*   Complex industrial automation and process control

## ✨ Core Features

*   **⏱️ High-Resolution Clock:** A configurable master clock that can tick hundreds of times per second, suitable for real-time applications (`Low`, `Medium`, `High`, `Ultra`).
*   **🔄 Configurable Phase Cycle:** Define a custom sequence of logical phases (e.g., "input", "logic", "render") that execute on every single tick, giving your application a predictable heartbeat.
*   **📣 Powerful Event System:** A fully asynchronous, multi-channel event bus. Your application's components are completely decoupled and communicate by subscribing to and broadcasting strongly-typed events.
*   **🗓️ Rich Task Scheduling:**
    *   **Intervals:** Run tasks at a regular interval (e.g., every 5 seconds).
    *   **Conditionals:** Trigger tasks only when a specific condition in your application's state is met.
    *   **Lifecycle Loops:** Define complex, multi-step automations that execute a sequence of actions over time.
*   **🦀 Safe Concurrency:** Built from the ground up with `tokio` and thread-safe primitives (`Arc`, `RwLock`, `SlotMap`) to ensure your application is robust and free from data races.
*   **🐚 Interactive Shell:** Comes with `hypershell`, a fully interactive command-line application for live testing and interaction with a running Hyperclock engine.

## 🏁 Getting Started: The Interactive Shell

The fastest way to experience Hyperclock is to run the `hypershell`.

### Prerequisites

*   Rust and Cargo (2021 edition or later)
*   Git

### Running the Shell

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-username/hyperclock-project.git
    cd hyperclock-project
    ```

2.  **Run `hypershell`:**
    The `-p` flag tells Cargo to run the `rdx-hypershell` package from within the workspace.
    ```bash
    cargo run -p rdx-hypershell
    ```

### Interactive Session Example

Once running, you can interact with the live engine.

```sh
# The shell starts up with the banner and prompt
>> 

# Add a task that will fire every 3 seconds
>> add interval 3
--> Added interval listener with handle: #0

<-- [SYSTEM EVENT] ListenerAdded { id: ListenerId(0v1) }
>> 

# See the list of active listeners
>> list
Active Listeners:
  Handle #0: ListenerId(0v1)
>> 

# After 3 seconds, the background engine fires the task!
<-- [INTERVAL TASK] A 3-second interval fired!

# Remove the listener using its handle
>> remove interval 0
--> Listener successfully removed.

<-- [SYSTEM EVENT] ListenerRemoved { id: ListenerId(0v1) }
>>

# The task will no longer fire. Now exit the shell.
>> exit
Exiting hypershell...

# The engine shuts down gracefully
INFO [Hyper Engine] has shut down.
```

## 📚 Usage as a Library

To use Hyperclock in your own project, add `rdx-hyperclock` as a dependency.

### Basic Engine Setup

Here's how to initialize and run the engine in your own `main.rs`.

```rust
use rdx_hyperclock::prelude::*;
use std::time::Duration;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create a configuration for the engine.
    let config = HyperclockConfig {
        resolution: ClockResolution::High, // 60 ticks per second
        ..Default::default()
    };

    // 2. Create the engine instance.
    let engine = HyperclockEngine::new(config);

    // 3. Register your application's logic.
    engine.on_interval(
        PhaseId(0),
        Duration::from_secs(1),
        || println!("One second has passed!")
    ).await;

    // 4. Run the engine. This is a blocking call.
    engine.run().await?;

    Ok(())
}
```

## 🔧 Configuration

The engine's behavior can be configured via a TOML file. You would create a `config.toml` file and load it using the `config` crate in your application.

**Example `config.toml`:**
```toml
# Set the master clock to run at 120 ticks per second.
resolution = "ultra"

# Define a 3-phase cycle for our game loop.
[[phases]]
id = 0
label = "input_processing"

[[phases]]
id = 1
label = "game_logic_and_physics"

[[phases]]
id = 2
label = "render_prep"
```

## 🛣️ The Road Ahead

Hyperclock is under active development. Key features planned for the future include:
*   **Dynamic Engine Control:** Refactoring the engine to use a Command Pattern for extreme robustness and thread safety.
*   **Snapshot & Rehydration:** The ability to save the engine's entire state to a file and restore it later.
*   **Crontab-style Task Loading:** Defining tasks and listeners in a data file that can be hot-reloaded while the engine is running.
*   **Web Dashboard:** A simple web interface for monitoring and controlling a running Hyperclock instance.

## 🗃️ Rustadex Codex

Hyperclock & Hypershell are part of the [Rustadex](https://github.com/rustadex/rustadex) (RDX) Codex. Library of Rust-based tools and parts for general junkyard engineering. Build a rocket.

<p align="center">
  <img src="https://raw.githubusercontent.com/rustadex/rustadex/main/.github/assets/rdx.png" alt="placerholder" width="300">
</p>


## 🤝 Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

## 📜 License

This project is dual-licensed under the terms of both the **MIT license** and the **Apache License, Version 2.0**.
```
