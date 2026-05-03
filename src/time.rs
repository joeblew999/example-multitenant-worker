//! Wall-clock helpers. Trait-based so tests can inject a fixed clock.

use std::sync::Arc;

pub trait Clock: Send + Sync + 'static {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> i64;

    fn now_unix_seconds(&self) -> i64 {
        self.now_ms() / 1000
    }
}

#[derive(Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    #[cfg(target_arch = "wasm32")]
    fn now_ms(&self) -> i64 {
        worker::Date::now().as_millis() as i64
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn now_ms(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_millis() as i64
    }
}

pub type SharedClock = Arc<dyn Clock>;

#[cfg(test)]
pub struct FixedClock {
    pub at_ms: std::sync::atomic::AtomicI64,
}

#[cfg(test)]
impl FixedClock {
    pub fn new(now_ms: i64) -> Arc<Self> {
        Arc::new(Self {
            at_ms: std::sync::atomic::AtomicI64::new(now_ms),
        })
    }

    pub fn advance(&self, by_ms: i64) {
        self.at_ms
            .fetch_add(by_ms, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.at_ms.load(std::sync::atomic::Ordering::Relaxed)
    }
}
