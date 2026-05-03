//! Shared application state: stores, providers, keyring, clock, and
//! config. Each service holds an `Arc<AppState<R, B>>` and reads through
//! it.

use std::sync::Arc;

use crate::auth::Keyring;
use crate::time::SharedClock;

/// Tunables sourced from environment vars (or defaults).
#[derive(Clone, Debug)]
pub struct Config {
    pub session_ttl_seconds: i64,
    pub invitation_ttl_seconds: i64,
    pub password_reset_ttl_seconds: i64,
    pub email_verify_ttl_seconds: i64,
    pub sso_state_ttl_seconds: i64,
    pub enforce_email_verification: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session_ttl_seconds: 86_400,
            invitation_ttl_seconds: 7 * 86_400,
            password_reset_ttl_seconds: 15 * 60,
            email_verify_ttl_seconds: 86_400,
            sso_state_ttl_seconds: 10 * 60,
            enforce_email_verification: false,
        }
    }
}

pub struct AppState<R, B> {
    pub repo: R,
    pub billing: B,
    pub keyring: Arc<Keyring>,
    pub clock: SharedClock,
    pub config: Config,
}

pub type SharedState<R, B> = Arc<AppState<R, B>>;
