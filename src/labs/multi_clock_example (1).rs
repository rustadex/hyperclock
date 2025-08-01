//! Example of a multi‑clock engine with three temporal streams: prime, lookahead and
//! memory. Each clock runs at the same frequency but with a different offset.
//!
//! `prime` ticks at time t.
//! `lookahead` ticks at time t + period (one tick ahead of prime).
//! `memory` ticks at time t (synchronized with prime), but you can interpret its
//! events as operating on data from t − period.
//!
//! Each clock holds its own buffer of events. In this example the buffers are
//! simple counters that are incremented on each tick and logged to stdout.
//! You can replace the callback bodies with whatever logic your application
//! requires.

use std::time::Duration;
use tokio::time::{interval_at, Instant};

/// A temporal clock with its own tick interval, start offset and task.
struct TemporalClock {
    name: &'static str,
    interval: Duration,
    offset: Duration,
    buffer: u64,
}

impl TemporalClock {
    fn new(name: &'static str, interval: Duration, offset: Duration) -> Self {
        Self {
            name,
            interval,
            offset,
            buffer: 0,
        }
    }

    /// Spawn the clock loop. On each tick it runs the provided callback,
    /// passing a mutable reference to itself.
    async fn run<F>(mut self, mut callback: F)
    where
        F: FnMut(&mut TemporalClock) + Send + 'static,
    {
        let start = Instant::now() + self.offset;
        let mut ticker = interval_at(start, self.interval);
        loop {
            ticker.tick().await;
            callback(&mut self);
        }
    }
}

/// Example program entry point. Starts three clocks with identical periods but
/// different offsets and tasks.
#[tokio::main]
async fn main() {
    // Each clock ticks once per second. Lookahead clock starts one tick later.
    let period = Duration::from_secs(1);

    let prime_clock = TemporalClock::new("prime", period, Duration::from_secs(0));
    let lookahead_clock = TemporalClock::new("lookahead", period, period);
    let memory_clock = TemporalClock::new("memory", period, Duration::from_secs(0));

    // Spawn each clock on its own task. The callbacks update the buffer and log.
    tokio::spawn(prime_clock.run(|clock| {
        clock.buffer += 1;
        println!("{} tick: buffer = {}", clock.name, clock.buffer);
    }));

    tokio::spawn(lookahead_clock.run(|clock| {
        clock.buffer += 1;
        println!("{} tick: buffer = {}", clock.name, clock.buffer);
    }));

    tokio::spawn(memory_clock.run(|clock| {
        clock.buffer += 1;
        println!("{} tick: buffer = {}", clock.name, clock.buffer);
    }));

    // Keep the main task alive indefinitely.
    futures::future::pending::<()>().await;
}