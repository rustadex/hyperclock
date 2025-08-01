//! Simulation of three temporal buffers: future, prime and memory, with
//! decaying mirror generation and biased prime selection. This code does
//! **not** use asynchronous clocks; instead it steps through ticks in a
//! synchronous loop. It can run for a fixed number of ticks or a duration.
//!
//! Future (t+1) is generated one tick ahead of prime using a decaying mirror
//! scheme: given the previous value, there is a sequence of probabilities
//! [80%, 70%, 60%, 50%] that the next value will repeat the previous value.
//! When a new value is chosen, the decay resets; if the same value is chosen
//! again, the decay restarts from 70%. After the 50% stage, a new number is
//! forced and the decay resets to 0.
//!
//! Prime (t) selects a random candidate 0–9 and applies a bias. If the
//! candidate and the predicted future both satisfy the bias, prime picks
//! its own candidate. If the candidate fails the bias but the future value
//! satisfies it, prime takes the future value with 75% probability. If both
//! fail the bias, prime flips a coin to choose between candidate and
//! prediction. If candidate == prediction, it is always chosen.
//!
//! Memory (t−1) echoes the prime value at t if it satisfies the bias; otherwise
//! memory skips writing for that tick.

use rand::Rng;
use std::time::{Duration, Instant};

/// Bias types for prime selection.
#[derive(Clone)]
pub enum Bias {
    Even,
    Odd,
    Unique,
    /// Custom predicate. The boxed function should return true if the
    /// candidate satisfies the bias.
    Custom(Box<dyn Fn(u8) -> bool + Send + Sync>),
    /// No bias: always valid.
    None,
}

impl Bias {
    /// Check whether a number satisfies this bias, given the previous prime
    /// value for the Unique case.
    fn check(&self, value: u8, prev_prime: Option<u8>) -> bool {
        match self {
            Bias::Even => value % 2 == 0,
            Bias::Odd => value % 2 == 1,
            Bias::Unique => match prev_prime {
                Some(prev) => value != prev,
                None => true,
            },
            Bias::Custom(f) => f(value),
            Bias::None => true,
        }
    }
}

/// Generator for the future buffer using the decaying mirror scheme.
pub struct DecayingMirror {
    prev: Option<u8>,
    stage: usize,
    // probabilities for repeating the previous value (in percent).
    probabilities: [u8; 4],
    rng: rand::rngs::ThreadRng,
}

impl DecayingMirror {
    pub fn new() -> Self {
        Self {
            prev: None,
            stage: 0,
            probabilities: [80, 70, 60, 50],
            rng: rand::thread_rng(),
        }
    }

    /// Generate the next value. Values are in 0..=9.
    pub fn next(&mut self) -> u8 {
        let new_value;
        if let Some(prev) = self.prev {
            let chance = self.probabilities[self.stage] as u32;
            let roll = self.rng.gen_range(0..100);
            if roll < chance {
                // repeat previous value
                new_value = prev;
                // advance or reset stage
                if self.stage + 1 < self.probabilities.len() {
                    self.stage += 1;
                } else {
                    // reached 50%, next call should force a new number
                    self.stage = 0;
                }
            } else {
                // choose a different value uniformly
                loop {
                    let candidate = self.rng.gen_range(0..10) as u8;
                    if candidate != prev {
                        new_value = candidate;
                        break;
                    }
                }
                // reset stage on new number
                self.stage = 0;
            }
        } else {
            // first value
            new_value = self.rng.gen_range(0..10) as u8;
            self.stage = 0;
        }
        self.prev = Some(new_value);
        new_value
    }
}

/// Simulation of the triple‑buffer system.
pub struct Simulation {
    decaying: DecayingMirror,
    bias: Bias,
    /// predicted future values (t+1); index 0 holds the prediction for t=1.
    future: Vec<Option<u8>>,
    /// prime values at time t.
    prime: Vec<Option<u8>>,
    /// memory values at time t (echo of prime at t−1).
    memory: Vec<Option<u8>>,
    rng: rand::rngs::ThreadRng,
}

impl Simulation {
    pub fn new(bias: Bias) -> Self {
        Self {
            decaying: DecayingMirror::new(),
            bias,
            future: Vec::new(),
            prime: Vec::new(),
            memory: Vec::new(),
            rng: rand::thread_rng(),
        }
    }

    /// Run the simulation for a number of ticks. At each tick t (starting at 0),
    /// future[t+1] is generated, then prime[t] is decided based on bias and
    /// future[t], and memory[t−1] echoes prime[t] if bias is satisfied.
    pub fn run_ticks(&mut self, ticks: usize) {
        // Ensure buffers are long enough
        self.future.resize(ticks + 1, None);
        self.prime.resize(ticks, None);
        self.memory.resize(ticks, None);

        for t in 0..ticks {
            // generate prediction for next tick (t+1)
            let predicted = self.decaying.next();
            self.future[t + 1] = Some(predicted);

            if t == 0 {
                // prime and memory start empty; only generate prediction
                continue;
            }

            // generate prime candidate
            let candidate = self.rng.gen_range(0..10) as u8;
            let prev_prime = self.prime[t - 1];
            let candidate_valid = self.bias.check(candidate, prev_prime);
            let future_val = self.future[t].unwrap();
            let future_valid = self.bias.check(future_val, prev_prime);

            let chosen = if candidate == future_val {
                candidate
            } else if !candidate_valid && future_valid {
                // prefer future 75% when candidate invalid, future valid
                if self.rng.gen_bool(0.75) {
                    future_val
                } else {
                    candidate
                }
            } else if candidate_valid && !future_valid {
                candidate
            } else if candidate_valid && future_valid {
                candidate
            } else {
                // both invalid, flip coin
                if self.rng.gen_bool(0.5) {
                    candidate
                } else {
                    future_val
                }
            };

            self.prime[t] = Some(chosen);
            // memory echoes prime[t] at t−1 if bias satisfied
            if self.bias.check(chosen, prev_prime) {
                self.memory[t - 1] = Some(chosen);
            }
        }
    }

    /// Print the buffers for debugging.
    pub fn print_buffers(&self) {
        println!("t  | future | prime | memory");
        let n = self.prime.len();
        for t in 0..n {
            let fut = self.future.get(t + 1).and_then(|x| *x);
            let pri = self.prime[t];
            let mem = if t == 0 { None } else { self.memory[t - 1] };
            println!(
                "{:2} |   {:?}   |  {:?}  |   {:?}",
                t,
                fut,
                pri,
                mem
            );
        }
    }
}

/// Example usage: simulate 10 ticks with an even bias and print the buffers.
fn main() {
    let mut sim = Simulation::new(Bias::Even);
    sim.run_ticks(10);
    sim.print_buffers();
}