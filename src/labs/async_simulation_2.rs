//! Asynchronous simulation of the three‑buffer system (future, prime, memory)
//! using a multi‑clock scheme. Each component runs as a scheduled job with its
//! own offset on a common tick interval. The future generator runs first,
//! producing predictions for the next tick; the prime selector runs one
//! interval later, using the previously predicted value; the memory updater
//! runs another interval later to echo prime’s value if the bias is satisfied.
//!
//! Requires the `tokio` runtime. Add this to your Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! rand = "0.8"
//! tokio = { version = "1", features = ["time", "macros"] }
//! ```

use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{sleep_until, Instant as TokioInstant};

use crate::{Bias, DecayingMirror};

/// Shared simulation state used by all scheduled jobs. The vectors contain
/// `Option<u8>` entries for each tick. `future[tick + 1]` stores the
/// prediction for tick `tick`. `prime[tick]` holds the chosen prime value at
/// that tick. `memory[tick]` echoes `prime[tick + 1]` if it satisfies the bias.
struct SimulationState {
    decaying: DecayingMirror,
    bias: Bias,
    future: Vec<Option<u8>>,
    prime: Vec<Option<u8>>,
    memory: Vec<Option<u8>>,
    future_tick: usize,
    prime_tick: usize,
    memory_tick: usize,
    max_ticks: usize,
    rng: rand::rngs::ThreadRng,
}

impl SimulationState {
    fn new(max_ticks: usize, bias: Bias) -> Self {
        Self {
            decaying: DecayingMirror::new(),
            bias,
            future: vec![None; max_ticks + 2],
            prime: vec![None; max_ticks + 1],
            memory: vec![None; max_ticks + 1],
            future_tick: 0,
            prime_tick: 0,
            memory_tick: 0,
            max_ticks,
            rng: rand::thread_rng(),
        }
    }
}

/// A scheduled job that runs at a fixed interval with an initial offset.
struct Job {
    interval: Duration,
    offset: Duration,
    next_run: TokioInstant,
    callback: Box<dyn FnMut() -> bool + Send + 'static>,
}

impl Job {
    fn new<F>(interval: Duration, offset: Duration, callback: F) -> Self
    where
        F: FnMut() -> bool + Send + 'static,
    {
        let now = TokioInstant::now();
        Self {
            interval,
            offset,
            next_run: now + offset,
            callback: Box::new(callback),
        }
    }
}

/// A simple scheduler that executes jobs at their scheduled times. Each job
/// returns `true` if it should continue, or `false` to remove itself. The
/// scheduler stops when all jobs are finished.
pub struct Scheduler {
    jobs: Vec<Job>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn add_job(&mut self, job: Job) {
        self.jobs.push(job);
    }

    pub async fn run(mut self) {
        loop {
            // find the next job to run
            if let Some((idx, next_time)) = self
                .jobs
                .iter()
                .enumerate()
                .map(|(i, job)| (i, job.next_run))
                .min_by_key(|&(_, t)| t)
            {
                sleep_until(next_time).await;
                // run the job; remove if finished
                let keep = {
                    let job = &mut self.jobs[idx];
                    (job.callback)()
                };
                if keep {
                    let job = &mut self.jobs[idx];
                    job.next_run += job.interval;
                } else {
                    self.jobs.remove(idx);
                }
            } else {
                break;
            }
        }
    }
}

/// Launch the asynchronous simulation using three scheduled jobs. The `period`
/// defines the duration between prime ticks. Future ticks run immediately,
/// prime ticks run after one period, and memory ticks run after two periods.
pub async fn run_async_simulation(max_ticks: usize, period: Duration, bias: Bias) {
    let state = Arc::new(Mutex::new(SimulationState::new(max_ticks, bias)));

    let mut scheduler = Scheduler::new();

    // Job for generating future predictions (lookahead).
    {
        let state = Arc::clone(&state);
        scheduler.add_job(Job::new(period, Duration::from_secs(0), move || {
            let mut sim = state.lock().unwrap();
            if sim.future_tick <= sim.max_ticks {
                let predicted = sim.decaying.next();
                if sim.future_tick + 1 < sim.future.len() {
                    sim.future[sim.future_tick + 1] = Some(predicted);
                }
                println!("[future  ] t={} predicted {}", sim.future_tick, predicted);
                sim.future_tick += 1;
                true
            } else {
                false
            }
        }));
    }

    // Job for computing prime values.
    {
        let state = Arc::clone(&state);
        scheduler.add_job(Job::new(period, period, move || {
            let mut sim = state.lock().unwrap();
            if sim.prime_tick < sim.max_ticks {
                let t = sim.prime_tick;
                let predicted = sim.future[t + 1].expect("prediction missing");
                let candidate = sim.rng.gen_range(0..10) as u8;
                let prev_prime = if t > 0 { sim.prime[t - 1] } else { None };
                let candidate_valid = sim.bias.check(candidate, prev_prime);
                let future_valid = sim.bias.check(predicted, prev_prime);
                let chosen = if candidate == predicted {
                    candidate
                } else if !candidate_valid && future_valid {
                    if sim.rng.gen_bool(0.75) {
                        predicted
                    } else {
                        candidate
                    }
                } else if candidate_valid && !future_valid {
                    candidate
                } else if candidate_valid && future_valid {
                    candidate
                } else {
                    if sim.rng.gen_bool(0.5) {
                        candidate
                    } else {
                        predicted
                    }
                };
                sim.prime[t] = Some(chosen);
                println!("[prime   ] t={} prime {} (pred={}, cand={})", t, chosen, predicted, candidate);
                if t > 0 && sim.bias.check(chosen, prev_prime) {
                    sim.memory[t - 1] = Some(chosen);
                    println!("[memory  ] t={} echo {}", t - 1, chosen);
                }
                sim.prime_tick += 1;
                true
            } else {
                false
            }
        }));
    }

    // Job for reporting memory values.
    {
        let state = Arc::clone(&state);
        scheduler.add_job(Job::new(period, period * 2, move || {
            let mut sim = state.lock().unwrap();
            if sim.memory_tick < sim.max_ticks {
                let t = sim.memory_tick;
                if let Some(val) = sim.memory[t] {
                    println!("[report  ] t={} memory={}", t, val);
                } else {
                    println!("[report  ] t={} memory=None", t);
                }
                sim.memory_tick += 1;
                true
            } else {
                false
            }
        }));
    }

    // Run the scheduler
    scheduler.run().await;

    // After simulation, print final buffers.
    let sim = state.lock().unwrap();
    println!("\nFinal buffers:");
    println!("t  | future | prime | memory");
    for t in 0..sim.max_ticks {
        let fut = sim.future.get(t + 1).and_then(|&x| x);
        let pri = sim.prime[t];
        let mem = sim.memory[t];
        println!("{:2} |   {:?}   |  {:?}  |   {:?}", t, fut, pri, mem);
    }
}