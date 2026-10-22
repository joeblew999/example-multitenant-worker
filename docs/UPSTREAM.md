# Upstream dependency tracking (`cedar` branch)

This fork has two upstreams: **connyay's original** (code) and
**cf-connectrpc-middleware** (crates). A pin is a **snapshot**, not a freeze.
This file is the map.

_Last checked: 2026-10-22._ Cross-repo rollout tracker: [cf-connectrpc-middleware#4](https://github.com/joeblew999/cf-connectrpc-middleware/issues/4).

## Branches

| Branch | Tracks | State |
|---|---|---|
| `main` | [connyay/example-multitenant-worker](https://github.com/connyay/example-multitenant-worker) `main` | identical (synced 2026-10-22); upstream is on connectrpc 0.4 |
| `cedar` | our integration: connyay's app + the cf-connectrpc-middleware crates | connectrpc 0.9.1 (#1) |

## What `cedar` depends on

| Dep | We pin | Latest published | Source | Notes |
|---|---|---|---|---|
| cf-connectrpc-middleware crates (`connectrpc-cedar`, `-cedar-interceptor`, `-cf-tracing`, `-cf-rate-limit`, `-cf-metrics`) | **path** `../../crates/*` | its `main` | [joeblew999/cf-connectrpc-middleware](https://github.com/joeblew999/cf-connectrpc-middleware) | Builds **only** from `cf-connectrpc-middleware/.src/example-multitenant-worker` (`mise run src:clone` there). It always follows the middleware's `main`, so bump `connectrpc` here in step with it. |
| `connectrpc` | **0.9.1** (`json`) | 0.9.1 (2026-09-21) | [anthropics/connect-rust](https://github.com/anthropics/connect-rust) | Must match the middleware's line. Without `json` (opt-in since 0.8), JSON is **HTTP 415**. |
| `connectrpc-build` | **0.9.0** | 0.9.0 | anthropics/connect-rust | |
| `buffa` / `buffa-types` | **0.9.2** | 0.9.2 (2026-09-03) | [anthropics/buffa](https://github.com/anthropics/buffa) | Lockstep with `connectrpc`. 0.9 generates some message fields as `MessageField<T, Inline<T>>`, so helpers that build them are generic over the storage. |
| `worker` / `worker-macros` | **0.8.7** (`http`, `d1`) | 0.8.7 (2026-09-25) | [cloudflare/workers-rs](https://github.com/cloudflare/workers-rs) | Needs Rust ≥ 1.91; `worker-build@^0.8` (= 0.8.7) needs wasm-bindgen ≥ 0.2.125 (on 0.2.129). |
| `libmacaroon` | **0.2** (lock 0.2.1) | **0.3.0** ⚠️ | [crates.io/libmacaroon](https://crates.io/crates/libmacaroon) | One minor behind. Evaluate together with orangevault-admin. |

## Upgrade order

1. cf-connectrpc-middleware bumps first (it's the hub).
2. Then here: bump `connectrpc` / `buffa` / `connectrpc-build` / `worker` to the same line, then `cargo update`, `cargo test`, clippy `-D warnings` (wasm32 + native), and `worker-build --release`.
3. Pulling connyay's upstream changes: sync `main`, then rebase/merge `cedar` onto it.
