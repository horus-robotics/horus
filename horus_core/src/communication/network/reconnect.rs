/// Reconnection logic with exponential backoff for network failures
///
/// Production-grade reconnection handling for robust distributed systems
use std::time::Duration;

const INITIAL_BACKOFF: Duration = Duration::from_millis(100);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const BACKOFF_MULTIPLIER: f64 = 2.0;
const MAX_RETRIES: usize = 10; // 0 means infinite retries

/// Reconnection strategy with exponential backoff
#[derive(Debug, Clone)]
pub struct ReconnectStrategy {
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub multiplier: f64,
    pub max_retries: usize, // 0 = infinite
    pub jitter: bool,       // Add random jitter to avoid thundering herd
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self {
            initial_backoff: INITIAL_BACKOFF,
            max_backoff: MAX_BACKOFF,
            multiplier: BACKOFF_MULTIPLIER,
            max_retries: MAX_RETRIES,
            jitter: true,
        }
    }
}

impl ReconnectStrategy {
    /// Create a new reconnection strategy
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strategy for production use (longer backoffs, infinite retries)
    pub fn production() -> Self {
        Self {
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(60),
            multiplier: 2.0,
            max_retries: 0, // Never give up
            jitter: true,
        }
    }

    /// Create a strategy for testing (short backoffs, limited retries)
    pub fn testing() -> Self {
        Self {
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(500),
            multiplier: 1.5,
            max_retries: 3,
            jitter: false,
        }
    }

    /// Calculate backoff delay for the given attempt
    pub fn backoff_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        // Calculate exponential backoff
        let delay_ms = self.initial_backoff.as_millis() as f64
            * self.multiplier.powi((attempt - 1) as i32);

        let delay = Duration::from_millis(delay_ms as u64);
        let capped_delay = delay.min(self.max_backoff);

        // Add jitter (±20%) to avoid thundering herd
        if self.jitter {
            use std::collections::hash_map::RandomState;
            use std::hash::BuildHasher;

            let hash_value = RandomState::new().hash_one(std::thread::current().id());
            let jitter_factor = 0.8 + (hash_value % 40) as f64 / 100.0; // 0.8 to 1.2

            Duration::from_millis((capped_delay.as_millis() as f64 * jitter_factor) as u64)
        } else {
            capped_delay
        }
    }

    /// Check if we should retry after this many attempts
    pub fn should_retry(&self, attempt: usize) -> bool {
        self.max_retries == 0 || attempt < self.max_retries
    }
}

/// Connection state tracker for monitoring
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    Connected,
    Reconnecting { attempt: usize },
    Failed { attempts: usize },
}

/// Reconnection context that tracks state
pub struct ReconnectContext {
    pub strategy: ReconnectStrategy,
    pub attempt: usize,
    pub health: ConnectionHealth,
}

impl ReconnectContext {
    pub fn new(strategy: ReconnectStrategy) -> Self {
        Self {
            strategy,
            attempt: 0,
            health: ConnectionHealth::Connected,
        }
    }

    /// Start a reconnection attempt
    pub fn begin_reconnect(&mut self) {
        self.attempt += 1;
        self.health = ConnectionHealth::Reconnecting {
            attempt: self.attempt,
        };
    }

    /// Mark connection as successful (resets attempt counter)
    pub fn mark_connected(&mut self) {
        self.attempt = 0;
        self.health = ConnectionHealth::Connected;
    }

    /// Mark connection as failed
    pub fn mark_failed(&mut self) {
        self.health = ConnectionHealth::Failed {
            attempts: self.attempt,
        };
    }

    /// Get the current backoff delay
    pub fn backoff_delay(&self) -> Duration {
        self.strategy.backoff_delay(self.attempt)
    }

    /// Check if we should continue retrying
    pub fn should_retry(&self) -> bool {
        self.strategy.should_retry(self.attempt)
    }

    /// Wait for the backoff period
    pub fn wait_backoff(&self) {
        let delay = self.backoff_delay();
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let strategy = ReconnectStrategy::testing();

        let delay1 = strategy.backoff_delay(1);
        let delay2 = strategy.backoff_delay(2);
        let delay3 = strategy.backoff_delay(3);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let strategy = ReconnectStrategy {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(5),
            multiplier: 2.0,
            max_retries: 0,
            jitter: false,
        };

        let delay = strategy.backoff_delay(100); // Very high attempt
        assert!(delay <= Duration::from_secs(5));
    }

    #[test]
    fn test_max_retries() {
        let strategy = ReconnectStrategy {
            initial_backoff: INITIAL_BACKOFF,
            max_backoff: MAX_BACKOFF,
            multiplier: BACKOFF_MULTIPLIER,
            max_retries: 3,
            jitter: false,
        };

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
        assert!(!strategy.should_retry(4));
    }

    #[test]
    fn test_infinite_retries() {
        let strategy = ReconnectStrategy::production();

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(100));
        assert!(strategy.should_retry(1000));
    }

    #[test]
    fn test_context_state_transitions() {
        let mut ctx = ReconnectContext::new(ReconnectStrategy::testing());

        assert_eq!(ctx.health, ConnectionHealth::Connected);
        assert_eq!(ctx.attempt, 0);

        ctx.begin_reconnect();
        assert_eq!(ctx.health, ConnectionHealth::Reconnecting { attempt: 1 });
        assert_eq!(ctx.attempt, 1);

        ctx.mark_connected();
        assert_eq!(ctx.health, ConnectionHealth::Connected);
        assert_eq!(ctx.attempt, 0);

        ctx.begin_reconnect();
        ctx.mark_failed();
        assert_eq!(ctx.health, ConnectionHealth::Failed { attempts: 1 });
    }
}
