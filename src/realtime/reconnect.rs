//! Exponential-backoff reconnect policy.

use std::time::Duration;

/// Options controlling exponential-backoff reconnect behaviour.
#[derive(Debug, Clone, Copy)]
pub struct ReconnectOptions {
    /// Delay before the first retry.
    pub initial_delay: Duration,
    /// Upper bound on any single delay.
    pub max_delay: Duration,
    /// Multiplier applied to the delay after each attempt.
    pub factor: f64,
    /// Fraction of jitter (0–1) applied to each delay.
    pub jitter: f64,
}

impl Default for ReconnectOptions {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            factor: 2.0,
            jitter: 0.2,
        }
    }
}

/// Computes exponential-backoff delays for reconnect attempts. Owns only the
/// timing policy; the client decides when to start and reset it.
#[derive(Debug)]
pub(crate) struct ReconnectPolicy {
    options: ReconnectOptions,
    attempt: u32,
}

impl ReconnectPolicy {
    pub(crate) fn new(options: ReconnectOptions) -> Self {
        Self {
            options,
            attempt: 0,
        }
    }

    /// Number of retries since the last successful connection.
    pub(crate) fn attempts(&self) -> u32 {
        self.attempt
    }

    /// Advance the attempt counter and return the delay before the next attempt.
    pub(crate) fn next_delay(&mut self) -> Duration {
        let initial = self.options.initial_delay.as_secs_f64();
        let max = self.options.max_delay.as_secs_f64();
        let base = (initial * self.options.factor.powi(self.attempt as i32)).min(max);
        self.attempt = self.attempt.saturating_add(1);
        // Deterministic jitter keeps the math testable while spreading load.
        let spread = base * self.options.jitter;
        let delay = base - spread / 2.0 + spread * self.pseudo_jitter();
        Duration::from_secs_f64(delay.max(0.0))
    }

    /// Clear backoff state after a successful connection or shutdown.
    pub(crate) fn reset(&mut self) {
        self.attempt = 0;
    }

    fn pseudo_jitter(&self) -> f64 {
        // Cheap, dependency-free spread derived from the attempt count.
        let x = (f64::from(self.attempt) * 12.9898).sin() * 43758.5453;
        x - x.floor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps_then_resets() {
        let mut policy = ReconnectPolicy::new(ReconnectOptions::default());
        let first = policy.next_delay();
        let second = policy.next_delay();
        assert!(second >= first, "delay should grow");
        for _ in 0..20 {
            let delay = policy.next_delay();
            // Within max + jitter spread.
            assert!(delay <= Duration::from_secs(35));
        }
        policy.reset();
        assert_eq!(policy.attempts(), 0);
    }
}
