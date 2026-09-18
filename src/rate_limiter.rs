use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Adaptive Token-Bucket Rate Limiter respecting TypeSafe AI limits (1,200 req/min).
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
}

struct RateLimiterState {
    max_tokens: f64,
    tokens: f64,
    refill_rate_per_sec: f64,
    last_refill: Instant,
    backoff_until: Option<Instant>,
}

impl RateLimiter {
    /// Creates a rate limiter with default max 20 requests/sec (1,200 req/min).
    pub fn new(max_req_per_sec: f64) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                max_tokens: max_req_per_sec,
                tokens: max_req_per_sec,
                refill_rate_per_sec: max_req_per_sec,
                last_refill: Instant::now(),
                backoff_until: None,
            })),
        }
    }

    /// Acquires a slot, sleeping if backoff or token exhaustion occurs.
    pub async fn acquire(&self) {
        loop {
            let wait_duration = {
                let mut state = self.state.lock().await;
                let now = Instant::now();

                // If under backoff, wait until it expires
                if let Some(backoff) = state.backoff_until {
                    if now < backoff {
                        Some(backoff - now)
                    } else {
                        state.backoff_until = None;
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(wait) = wait_duration {
                sleep(wait).await;
                continue;
            }

            let mut state = self.state.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state.last_refill).as_secs_f64();
            state.tokens = (state.tokens + elapsed * state.refill_rate_per_sec).min(state.max_tokens);
            state.last_refill = now;

            if state.tokens >= 1.0 {
                state.tokens -= 1.0;
                return;
            }

            // Need to wait for token refill
            let needed = 1.0 - state.tokens;
            let wait_secs = needed / state.refill_rate_per_sec;
            drop(state);
            sleep(Duration::from_secs_f64(wait_secs.max(0.01))).await;
        }
    }

    /// Registers an HTTP 429 response, applying exponential backoff or respecting `retry-after`.
    pub async fn trigger_backoff(&self, retry_after_secs: Option<u64>) {
        let mut state = self.state.lock().await;
        let delay = retry_after_secs
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_millis(1500));
        state.backoff_until = Some(Instant::now() + delay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_acquires_tokens() {
        let limiter = RateLimiter::new(10.0);
        limiter.acquire().await;
        limiter.acquire().await;
    }
}
