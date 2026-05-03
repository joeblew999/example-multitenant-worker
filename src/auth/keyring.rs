//! Macaroon root-key handling.
//!
//! A single 32-byte root key signs every token. We rely on the `purpose=`
//! caveat (and the verifier's exact-match satisfier) to keep token kinds
//! from being interchangeable: a session token can't be used to redeem an
//! invite, etc., because the verifier requires a specific purpose.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libmacaroon::MacaroonKey;

#[derive(Clone, Debug)]
pub enum KeyringError {
    InvalidLength(usize),
    InvalidBase64(String),
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyringError::InvalidLength(n) => write!(f, "invalid key length: {n} (need 32)"),
            KeyringError::InvalidBase64(s) => write!(f, "invalid key base64: {s}"),
        }
    }
}

impl std::error::Error for KeyringError {}

#[derive(Clone)]
pub struct Keyring {
    root: MacaroonKey,
}

impl Keyring {
    pub fn from_key(root: MacaroonKey) -> Self {
        Self { root }
    }

    /// Decode a base64-encoded 32-byte key. Accepts either standard or
    /// URL-safe alphabets, padded or unpadded — `wrangler secret put` and
    /// hand-edited TOML files have produced all four shapes in practice.
    pub fn from_base64(s: &str) -> Result<Self, KeyringError> {
        let s = s.trim();
        let bytes = STANDARD
            .decode(s)
            .or_else(|_| STANDARD_NO_PAD.decode(s))
            .or_else(|_| URL_SAFE.decode(s))
            .or_else(|_| URL_SAFE_NO_PAD.decode(s))
            .map_err(|e| KeyringError::InvalidBase64(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(KeyringError::InvalidLength(bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self {
            root: MacaroonKey::from(arr),
        })
    }

    /// Deterministic dev fallback used when `SESSION_KEY` is unset. NOT for
    /// production: a leaked binary recovers the key.
    pub fn dev_default() -> Self {
        Self {
            root: MacaroonKey::generate(b"workers-multitenant.dev-key.do-not-deploy"),
        }
    }

    pub fn root(&self) -> &MacaroonKey {
        &self.root
    }
}
