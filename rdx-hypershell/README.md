


# 🐚 hypershell
[![Crates.io Version](https://img.shields.io/crates/v/rdx-hyperclock.svg)](https://crates.io/crates/rdx-stderr)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/rdx-stderr.svg)](https://github.com/rustadex/hyperclock/blob/main/LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/msrv-1.70.0-blue.svg)](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html)




An interactive command-line interface for the [Hyperclock](https://github.com/rustadex/hyperclock) phase time engine.

`hypershell` provides a live, interactive REPL (Read-Eval-Print Loop) to start, configure, and interact with a running Hyperclock engine instance. It's the perfect tool for testing, debugging, and demonstrating the capabilities of the Hyperclock library.

## ✨ Features

*   **Live Interaction:** Add, remove, and list listeners in real-time.
*   **Event Monitoring:** See a live stream of system events from the engine.
*   **Dynamic Control:** Start and stop noisy event streams (like raw ticks) on the fly.
*   **Syntax Highlighting:** A colored prompt and user input for a pleasant user experience.
*   **Command History:** Standard shell features like command history are supported.

## 🚀 Installation & Usage

`hypershell` is part of the Hyperclock workspace. The recommended way to install it is from the Git repository.

1.  **Clone the project:**
    ```bash
    git clone https://github.com/your-username/hyperclock-project.git
    cd hyperclock-project
    ```
2.  **Install the shell binary:**
    This command will build the `hypershell` executable in release mode and install it into your Cargo bin path so you can run it from anywhere.
    ```bash
    cargo install --path rdx-hypershell
    ```
3.  **Run it!**
    ```bash
    hypershell
    ```

## 🎮 Interactive Session Example

```sh
# The shell starts up with a banner and prompt
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

# Exit the shell
>> exit
Exiting hypershell...
```

## 📋 Commands

| Command                 | Description                                                   | Example                             |
| :---------------------- | :------------------------------------------------------------ | :---------------------------------- |
| `add interval <S>`      | Adds an interval watcher that fires every `S` seconds.        | `add interval 10`                   |
| `list`                  | Lists all active listeners and their user-friendly handles.   | `list`                              |
| `remove interval <H>`   | Removes a watcher by its numeric handle (`#H`).               | `remove interval 0`                 |
| `start ticks`           | Begins printing a one-second summary of the raw tick stream.  | `start ticks`                       |
| `stop ticks`            | Stops printing the tick summary.                              | `stop ticks`                        |
| `help`                  | Displays this list of commands.                               | `help`                              |
| `exit`                  | Quits the shell.                                              | `exit`                              |

## 🤫 Quiet Mode

To start the shell without the ASCII art banner, set the `QUIET_MODE` environment variable.

```bash
QUIET_MODE=1 hypershell
```

## 📜 License

This project is dual-licensed under the terms of both the **MIT license** and the **Apache License, Version 2.0**.

