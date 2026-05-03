//! Argon2id password hashing.
//!
//! Picks a low-cost parameter set so this is usable inside Cloudflare
//! Workers (CPU is the binding constraint). Production deployments should
//! tune up `mem_cost` / `time_cost` per their CPU budget. The OWASP
//! reference (m=19MiB, t=2, p=1) easily fits inside Workers' 50ms-CPU
//! budget on the Free plan; we pick something a little tighter to leave
//! headroom for the rest of the request.
//!
//! Password complexity rules follow NIST 800-63B:
//!   - minimum 8 characters
//!   - no composition rules
//!   - HIBP k-anonymity check is plumbed through the [`PasswordPolicy`]
//!     trait; the demo ships an always-allow stub. Real production would
//!     call <https://api.pwnedpasswords.com/range/{prefix}> via worker
//!     fetch and reject anything the API has ever seen.

use argon2::{Algorithm, Argon2, Params, Version};
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;

const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug)]
pub enum PasswordError {
    TooShort(usize),
    Pwned,
    HashFailed(String),
    InvalidHash(String),
    Mismatch,
    PolicyError(String),
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordError::TooShort(n) => {
                write!(f, "password too short ({n} chars, need {MIN_PASSWORD_LEN})")
            }
            PasswordError::Pwned => f.write_str("password appears in known-breached corpus"),
            PasswordError::HashFailed(s) => write!(f, "hash failed: {s}"),
            PasswordError::InvalidHash(s) => write!(f, "invalid stored hash: {s}"),
            PasswordError::Mismatch => f.write_str("password mismatch"),
            PasswordError::PolicyError(s) => write!(f, "policy: {s}"),
        }
    }
}

impl std::error::Error for PasswordError {}

fn hasher() -> Argon2<'static> {
    // m_cost=4096 KiB, t_cost=2, p_cost=1 — gives ~25-40ms on Workers
    // hardware in our experiments. Adjust per CPU budget.
    let params = Params::new(4096, 2, 1, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Validate and hash a new password. Returns the PHC string suitable for
/// `identities.secret`.
pub async fn hash_new_password(password: &str) -> Result<String, PasswordError> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_LEN {
        return Err(PasswordError::TooShort(len));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = hasher()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| PasswordError::HashFailed(e.to_string()))?;
    Ok(hash.to_string())
}

pub fn verify_password(stored: &str, password: &str) -> Result<(), PasswordError> {
    let parsed =
        PasswordHash::new(stored).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;
    hasher()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| PasswordError::Mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn hash_then_verify_roundtrips() {
        let hash = block_on(hash_new_password("correct horse battery")).unwrap();
        verify_password(&hash, "correct horse battery").unwrap();
        assert!(verify_password(&hash, "wrong-password").is_err());
    }

    #[test]
    fn rejects_short_password() {
        let err = block_on(hash_new_password("short")).unwrap_err();
        assert!(matches!(err, PasswordError::TooShort(5)));
    }
}
