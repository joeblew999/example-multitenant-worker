//! Bridge `tracing` events to the JS `console` so `wrangler dev` and
//! `wrangler tail` show them. Idempotent — safe to call on every
//! request (the OnceLock guards the global subscriber install).
//!
//! Why a custom writer instead of `tracing-web`: this approach has
//! zero extra deps (tracing-subscriber is already in our Cargo.toml
//! for the dep tree's sake) and keeps the worker self-contained.

use std::sync::OnceLock;

/// Install once. Worker boot calls this; subsequent calls are no-ops.
pub fn init() {
    static GUARD: OnceLock<()> = OnceLock::new();
    GUARD.get_or_init(|| {
        // fmt with our console-bridging writer.
        //
        // - `.with_ansi(false)`: JS console doesn't render ANSI escapes
        // - `.with_target(true)`: keep the `connectrpc_cf_tracing` /
        //   `connectrpc_cf_rate_limit` module path so operators can filter
        // - `.without_time()`: **MANDATORY ON wasm32**. The default fmt
        //   timestamp calls `SystemTime::now()`, which panics on
        //   `wasm32-unknown-unknown` (no system clock). Without this,
        //   every first event panics with "unreachable". Cloudflare's
        //   `wrangler tail` and Logpush already attach their own
        //   timestamps anyway.
        let _ = tracing_subscriber::fmt()
            .with_writer(ConsoleMakeWriter)
            .with_ansi(false)
            .with_target(true)
            .without_time()
            // Default to INFO. Override via wrangler.toml `vars`:
            //   RUST_LOG = "info,connectrpc_cf_rate_limit=warn"
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();
    });
}

/// `MakeWriter` adapter that produces buffered console-bridging
/// writers, one per event.
struct ConsoleMakeWriter;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ConsoleMakeWriter {
    type Writer = ConsoleWriter;
    fn make_writer(&'a self) -> Self::Writer {
        ConsoleWriter { buf: Vec::new() }
    }
}

/// Buffers bytes from `fmt::Layer`, flushes them to `console.log` on
/// `flush()` and `drop()` — once per event. Calling console.log per
/// `write()` would split a single event across multiple console lines.
struct ConsoleWriter {
    buf: Vec<u8>,
}

impl std::io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buf.is_empty() {
            // Trailing newline is harmless in the console; strip it
            // anyway so multi-line events read cleanly.
            let s = String::from_utf8_lossy(&self.buf);
            let trimmed = s.trim_end_matches('\n');
            #[cfg(target_arch = "wasm32")]
            worker::console_log!("{}", trimmed);
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("{trimmed}");
            self.buf.clear();
        }
        Ok(())
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        let _ = std::io::Write::flush(self);
    }
}
