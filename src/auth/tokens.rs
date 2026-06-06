//! Mint / verify the four kinds of macaroon this service issues.
//!
//! # Security model
//!
//! Every authority-bearing claim (user, role, billing/org scope, purpose,
//! expiry, one-shot nonce, ...) is packed into a signed [`TokenClaims`]
//! protobuf message that becomes the macaroon **identifier**. The identifier
//! is folded into the root signature (`sig0 = HMAC(root_key, identifier)`),
//! so no holder can alter a claim without the root key.
//!
//! Caveats are deliberately NOT used to carry claims. A first-party caveat
//! can be appended by anyone holding a token — that is the defining macaroon
//! property — so caveats can only ever *attenuate*, never assert authority.
//! The previous design stored identity in caveats and read it back, which let
//! any token holder append `role=owner` / `user=<victim>` / `billing=<other>`
//! and silently escalate.
//!
//! The single legitimate caveat is `exp=<unix>`: expiry genuinely attenuates,
//! so a holder may append an *earlier* expiry and the effective expiry is the
//! earliest one (it can never be extended — appending a later `exp` is ignored
//! because we take the minimum). The verifier registers exactly one satisfier,
//! for well-formed `exp` caveats, so any *other* caveat (e.g. one an attacker
//! appended) is unsatisfied and verification fails.
//!
//! Verification is pure (no DB reads). One-shot tokens additionally carry a
//! `nonce` consumed via `Repo::consume_nonce` at the use RPC.

use buffa::Message as _;
use libmacaroon::{Caveat, Format, Macaroon, MacaroonError, Verifier};

use crate::domain::{
    AuthMethod, BillingAccountId, InvitationId, OrgId, Role, ScopeKind, TokenPurpose, UserId,
};
use crate::proto::workers::auth::v1::TokenClaims;

use super::keyring::Keyring;
use super::session::SessionContext;

#[derive(Debug)]
pub enum TokenError {
    Malformed(String),
    Expired,
    InvalidSignature,
    WrongPurpose {
        expected: TokenPurpose,
        found: String,
    },
    MissingClaim(&'static str),
    InvalidClaim(String),
    /// The token carried a caveat. Well-formed tokens have none; a present
    /// caveat means it was appended after minting (tampering) and the token
    /// is rejected.
    UnexpectedCaveat(String),
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::Malformed(s) => write!(f, "malformed token: {s}"),
            TokenError::Expired => write!(f, "token expired"),
            TokenError::InvalidSignature => write!(f, "invalid signature"),
            TokenError::WrongPurpose { expected, found } => {
                write!(f, "wrong token purpose: expected {expected}, got {found}")
            }
            TokenError::MissingClaim(c) => write!(f, "missing claim: {c}"),
            TokenError::InvalidClaim(c) => write!(f, "invalid claim: {c}"),
            TokenError::UnexpectedCaveat(c) => write!(f, "unexpected caveat on token: {c}"),
        }
    }
}

impl std::error::Error for TokenError {}

impl From<MacaroonError> for TokenError {
    fn from(e: MacaroonError) -> Self {
        match e {
            MacaroonError::InvalidSignature => TokenError::InvalidSignature,
            // The verifier registers no satisfiers, so any caveat present on
            // the token surfaces here — that only happens if a caveat was
            // appended post-mint.
            MacaroonError::CaveatNotSatisfied(s) => TokenError::UnexpectedCaveat(s),
            MacaroonError::DeserializationError(s) => TokenError::Malformed(s),
            other => TokenError::Malformed(other.to_string()),
        }
    }
}

const EXP_PREFIX: &[u8] = b"exp=";

/// Serialize signed claims into the macaroon identifier and add the `exp`
/// caveat. The token carries no other caveats.
fn pack(keyring: &Keyring, claims: &TokenClaims, exp_unix: i64) -> Result<String, TokenError> {
    let mut mac = Macaroon::create(
        Some("workers-multitenant"),
        keyring.root(),
        claims.encode_to_vec(),
    )?;
    mac.add_first_party_caveat(format!("exp={exp_unix}"))?;
    Ok(mac.serialize(Format::V2)?)
}

/// A caveat is satisfiable iff it is a well-formed `exp=<i64>`. Everything
/// else (including anything an attacker appends) is left unsatisfied, so the
/// verifier rejects the token.
fn parse_exp(caveat: &[u8]) -> Option<i64> {
    let rest = caveat.strip_prefix(EXP_PREFIX)?;
    std::str::from_utf8(rest).ok()?.parse::<i64>().ok()
}

/// Verify the signature, reject any non-`exp` caveat, enforce expiry, and
/// recover the signed claims. Effective expiry is the EARLIEST `exp` caveat,
/// so attenuation can only shorten a token's life, never extend it.
fn unpack(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<(TokenClaims, i64), TokenError> {
    let macaroon = Macaroon::deserialize(token)?;
    let mut verifier = Verifier::default();
    verifier.satisfy_general(|c: &[u8]| parse_exp(c).is_some());
    verifier.verify(&macaroon, keyring.root(), &[])?;

    let exp_unix = macaroon
        .caveats()
        .iter()
        .filter_map(|c| match c {
            Caveat::FirstParty(fp) => parse_exp(fp.predicate()),
            Caveat::ThirdParty(_) => None,
        })
        .min()
        .ok_or(TokenError::MissingClaim("exp"))?;
    if exp_unix <= now_unix {
        return Err(TokenError::Expired);
    }

    let claims = TokenClaims::decode_from_slice(macaroon.identifier())
        .map_err(|e| TokenError::Malformed(format!("claims decode: {e}")))?;
    Ok((claims, exp_unix))
}

fn check_purpose(claims: &TokenClaims, expected: TokenPurpose) -> Result<(), TokenError> {
    if claims.purpose != expected.as_str() {
        return Err(TokenError::WrongPurpose {
            expected,
            found: claims.purpose.clone(),
        });
    }
    Ok(())
}

fn require<'a>(value: &'a str, claim: &'static str) -> Result<&'a str, TokenError> {
    if value.is_empty() {
        Err(TokenError::MissingClaim(claim))
    } else {
        Ok(value)
    }
}

fn optional(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

#[derive(Clone, Debug)]
pub struct MintSessionInput<'a> {
    pub user_id: &'a UserId,
    pub email: &'a str,
    pub billing: &'a BillingAccountId,
    pub org: Option<&'a OrgId>,
    pub role: Role,
    pub auth_method: &'a AuthMethod,
    pub now_unix: i64,
    pub ttl_seconds: i64,
}

pub fn mint_session(keyring: &Keyring, input: MintSessionInput<'_>) -> Result<String, TokenError> {
    let claims = TokenClaims {
        purpose: TokenPurpose::Session.as_str().to_owned(),
        user_id: input.user_id.as_str().to_owned(),
        email: input.email.to_owned(),
        billing_account_id: input.billing.as_str().to_owned(),
        org_id: input.org.map(|o| o.as_str().to_owned()).unwrap_or_default(),
        role: input.role.as_str().to_owned(),
        auth_method: input.auth_method.encode(),
        ..Default::default()
    };
    pack(keyring, &claims, input.now_unix + input.ttl_seconds)
}

pub fn verify_session(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<SessionContext, TokenError> {
    let (claims, exp_unix) = unpack(keyring, now_unix, token)?;
    check_purpose(&claims, TokenPurpose::Session)?;

    let role = claims
        .role
        .parse::<Role>()
        .map_err(TokenError::InvalidClaim)?;
    let auth_method = AuthMethod::decode(&claims.auth_method)
        .ok_or_else(|| TokenError::InvalidClaim(format!("auth_method={}", claims.auth_method)))?;

    Ok(SessionContext {
        user: UserId::from(require(&claims.user_id, "user")?),
        email: require(&claims.email, "email")?.to_owned(),
        billing: BillingAccountId::from(require(&claims.billing_account_id, "billing")?),
        org: optional(&claims.org_id).map(OrgId::from),
        role,
        auth_method,
        exp_unix,
    })
}

#[derive(Clone, Debug)]
pub struct MintInvitationInput<'a> {
    pub invitation_id: &'a InvitationId,
    pub email: &'a str,
    pub scope_kind: ScopeKind,
    pub scope_id: &'a str,
    pub role: Role,
    pub required_idp: Option<&'a str>,
    pub nonce: &'a str,
    pub now_unix: i64,
    pub ttl_seconds: i64,
}

pub fn mint_invitation(
    keyring: &Keyring,
    input: MintInvitationInput<'_>,
) -> Result<String, TokenError> {
    let claims = TokenClaims {
        purpose: TokenPurpose::Invitation.as_str().to_owned(),
        nonce: input.nonce.to_owned(),
        email: input.email.to_owned(),
        invitation_id: input.invitation_id.as_str().to_owned(),
        scope_kind: input.scope_kind.as_str().to_owned(),
        scope_id: input.scope_id.to_owned(),
        role: input.role.as_str().to_owned(),
        required_idp: input.required_idp.unwrap_or_default().to_owned(),
        ..Default::default()
    };
    pack(keyring, &claims, input.now_unix + input.ttl_seconds)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvitationToken {
    pub invitation_id: InvitationId,
    pub email: String,
    pub scope_kind: ScopeKind,
    pub scope_id: String,
    pub role: Role,
    pub required_idp: Option<String>,
    pub nonce: String,
    pub exp_unix: i64,
}

pub fn verify_invitation(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<InvitationToken, TokenError> {
    let (claims, exp_unix) = unpack(keyring, now_unix, token)?;
    check_purpose(&claims, TokenPurpose::Invitation)?;

    let scope_kind = claims
        .scope_kind
        .parse::<ScopeKind>()
        .map_err(TokenError::InvalidClaim)?;
    let role = claims
        .role
        .parse::<Role>()
        .map_err(TokenError::InvalidClaim)?;

    Ok(InvitationToken {
        invitation_id: InvitationId::from(require(&claims.invitation_id, "invitation_id")?),
        email: require(&claims.email, "email")?.to_owned(),
        scope_kind,
        scope_id: require(&claims.scope_id, "scope_id")?.to_owned(),
        role,
        required_idp: optional(&claims.required_idp).map(str::to_owned),
        nonce: require(&claims.nonce, "nonce")?.to_owned(),
        exp_unix,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordResetToken {
    pub user_id: UserId,
    pub nonce: String,
    pub exp_unix: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmailVerifyToken {
    pub user_id: UserId,
    pub nonce: String,
    pub exp_unix: i64,
}

struct OneShotFields {
    user_id: UserId,
    nonce: String,
    exp_unix: i64,
}

fn mint_one_shot(
    keyring: &Keyring,
    purpose: TokenPurpose,
    user_id: &UserId,
    nonce: &str,
    now_unix: i64,
    ttl_seconds: i64,
) -> Result<String, TokenError> {
    let claims = TokenClaims {
        purpose: purpose.as_str().to_owned(),
        nonce: nonce.to_owned(),
        user_id: user_id.as_str().to_owned(),
        ..Default::default()
    };
    pack(keyring, &claims, now_unix + ttl_seconds)
}

fn verify_one_shot(
    keyring: &Keyring,
    expected: TokenPurpose,
    now_unix: i64,
    token: &str,
) -> Result<OneShotFields, TokenError> {
    let (claims, exp_unix) = unpack(keyring, now_unix, token)?;
    check_purpose(&claims, expected)?;
    Ok(OneShotFields {
        user_id: UserId::from(require(&claims.user_id, "user")?),
        nonce: require(&claims.nonce, "nonce")?.to_owned(),
        exp_unix,
    })
}

pub fn mint_password_reset(
    keyring: &Keyring,
    user_id: &UserId,
    nonce: &str,
    now_unix: i64,
    ttl_seconds: i64,
) -> Result<String, TokenError> {
    mint_one_shot(
        keyring,
        TokenPurpose::PasswordReset,
        user_id,
        nonce,
        now_unix,
        ttl_seconds,
    )
}

pub fn verify_password_reset(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<PasswordResetToken, TokenError> {
    let f = verify_one_shot(keyring, TokenPurpose::PasswordReset, now_unix, token)?;
    Ok(PasswordResetToken {
        user_id: f.user_id,
        nonce: f.nonce,
        exp_unix: f.exp_unix,
    })
}

pub fn mint_email_verify(
    keyring: &Keyring,
    user_id: &UserId,
    nonce: &str,
    now_unix: i64,
    ttl_seconds: i64,
) -> Result<String, TokenError> {
    mint_one_shot(
        keyring,
        TokenPurpose::EmailVerify,
        user_id,
        nonce,
        now_unix,
        ttl_seconds,
    )
}

pub fn verify_email_verify(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<EmailVerifyToken, TokenError> {
    let f = verify_one_shot(keyring, TokenPurpose::EmailVerify, now_unix, token)?;
    Ok(EmailVerifyToken {
        user_id: f.user_id,
        nonce: f.nonce,
        exp_unix: f.exp_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyring() -> Keyring {
        Keyring::dev_default()
    }

    fn sample_session(kr: &Keyring, now_unix: i64, ttl_seconds: i64) -> String {
        let user = UserId::from("u_1");
        let billing = BillingAccountId::from("ba_1");
        mint_session(
            kr,
            MintSessionInput {
                user_id: &user,
                email: "ada@example.com",
                billing: &billing,
                org: None,
                role: Role::Member,
                auth_method: &AuthMethod::Password,
                now_unix,
                ttl_seconds,
            },
        )
        .unwrap()
    }

    #[test]
    fn session_roundtrip() {
        let kr = keyring();
        let user = UserId::from("u_1");
        let billing = BillingAccountId::from("ba_1");
        let token = mint_session(
            &kr,
            MintSessionInput {
                user_id: &user,
                email: "ada@example.com",
                billing: &billing,
                org: None,
                role: Role::Owner,
                auth_method: &AuthMethod::Password,
                now_unix: 1_000,
                ttl_seconds: 60,
            },
        )
        .unwrap();
        let session = verify_session(&kr, 1_001, &token).unwrap();
        assert_eq!(session.user, user);
        assert_eq!(session.email, "ada@example.com");
        assert_eq!(session.billing, billing);
        assert_eq!(session.role, Role::Owner);
        assert_eq!(session.auth_method, AuthMethod::Password);
    }

    #[test]
    fn session_rejects_after_expiry() {
        let kr = keyring();
        let token = sample_session(&kr, 0, 10);
        assert!(matches!(
            verify_session(&kr, 100, &token),
            Err(TokenError::Expired)
        ));
    }

    #[test]
    fn session_rejects_wrong_key() {
        let kr = keyring();
        let other = Keyring::from_key(libmacaroon::MacaroonKey::generate(b"other"));
        let token = sample_session(&kr, 0, 60);
        assert!(verify_session(&other, 1, &token).is_err());
    }

    #[test]
    fn invitation_token_carries_required_idp() {
        let kr = keyring();
        let invitation_id = InvitationId::from("inv_1");
        let token = mint_invitation(
            &kr,
            MintInvitationInput {
                invitation_id: &invitation_id,
                email: "new@user.com",
                scope_kind: ScopeKind::Org,
                scope_id: "o_1",
                role: Role::Member,
                required_idp: Some("acme-okta"),
                nonce: "n1",
                now_unix: 0,
                ttl_seconds: 600,
            },
        )
        .unwrap();
        let parsed = verify_invitation(&kr, 1, &token).unwrap();
        assert_eq!(parsed.scope_kind, ScopeKind::Org);
        assert_eq!(parsed.scope_id, "o_1");
        assert_eq!(parsed.required_idp.as_deref(), Some("acme-okta"));
        assert_eq!(parsed.nonce, "n1");
    }

    #[test]
    fn cross_purpose_rejected() {
        let kr = keyring();
        let session_token = sample_session(&kr, 0, 60);
        // Treat the session token as an invitation — must fail on purpose.
        assert!(matches!(
            verify_invitation(&kr, 1, &session_token),
            Err(TokenError::WrongPurpose { .. })
        ));
    }

    // --- Regression tests for the macaroon-attenuation vulnerability ---
    //
    // These reproduce the original exploit: a token holder appends first-party
    // caveats (no key required) to escalate authority. The fix moves all
    // authority claims into the signed identifier; only an `exp` caveat is
    // satisfiable, so each of these appended-caveat attacks now FAILS.

    fn attacker_append(token: &str, extra: &[&str]) -> String {
        let mut mac = Macaroon::deserialize(token).unwrap();
        for c in extra {
            mac.add_first_party_caveat(*c).unwrap();
        }
        mac.serialize(Format::V2).unwrap()
    }

    #[test]
    fn session_rejects_appended_authority_caveats() {
        let kr = keyring();
        let token = sample_session(&kr, 1_000, 3_600);
        // The exact escalation that used to succeed.
        let forged = attacker_append(&token, &["role=owner", "user=u_victim", "billing=ba_victim"]);
        assert!(matches!(
            verify_session(&kr, 1_001, &forged),
            Err(TokenError::UnexpectedCaveat(_))
        ));
    }

    #[test]
    fn session_rejects_any_single_appended_caveat() {
        let kr = keyring();
        let token = sample_session(&kr, 1_000, 3_600);
        let forged = attacker_append(&token, &["role=owner"]);
        assert!(verify_session(&kr, 1_001, &forged).is_err());
    }

    #[test]
    fn one_shot_rejects_appended_user_override() {
        let kr = keyring();
        let user = UserId::from("u_self");
        // Attacker mints (legitimately) a reset for their own account...
        let token = mint_password_reset(&kr, &user, "nonce-self", 0, 600).unwrap();
        // ...then tries to retarget it at a victim by appending a caveat.
        let forged = attacker_append(&token, &["user=u_victim"]);
        assert!(matches!(
            verify_password_reset(&kr, 1, &forged),
            Err(TokenError::UnexpectedCaveat(_))
        ));
        // Sanity: the untampered token still verifies to the real subject.
        let ok = verify_password_reset(&kr, 1, &token).unwrap();
        assert_eq!(ok.user_id, user);
    }

    #[test]
    fn invitation_rejects_appended_scope_and_role_override() {
        let kr = keyring();
        let invitation_id = InvitationId::from("inv_1");
        let token = mint_invitation(
            &kr,
            MintInvitationInput {
                invitation_id: &invitation_id,
                email: "new@user.com",
                scope_kind: ScopeKind::Org,
                scope_id: "o_attacker",
                role: Role::Member,
                required_idp: None,
                nonce: "n1",
                now_unix: 0,
                ttl_seconds: 600,
            },
        )
        .unwrap();
        let forged = attacker_append(&token, &["scope_id=o_victim", "role=owner"]);
        assert!(verify_invitation(&kr, 1, &forged).is_err());
    }

    // --- `exp` is the one legitimately-attenuating caveat ---

    #[test]
    fn exp_caveat_cannot_be_extended() {
        let kr = keyring();
        // Token already expired (exp = 10).
        let token = sample_session(&kr, 0, 10);
        // Appending a far-future exp must NOT revive it — effective expiry is
        // the earliest exp caveat, so the original exp=10 still governs.
        let forged = attacker_append(&token, &["exp=9999999999"]);
        assert!(matches!(
            verify_session(&kr, 100, &forged),
            Err(TokenError::Expired)
        ));
    }

    #[test]
    fn exp_caveat_can_be_attenuated_shorter() {
        let kr = keyring();
        // Long-lived token (exp = 10_000).
        let token = sample_session(&kr, 0, 10_000);
        // A holder narrows it to expire at 500 — a legitimate attenuation.
        let narrowed = attacker_append(&token, &["exp=500"]);
        // Still valid before the tighter expiry...
        let s = verify_session(&kr, 100, &narrowed).unwrap();
        assert_eq!(s.exp_unix, 500);
        // ...and dead after it, even though the original exp is far away.
        assert!(matches!(
            verify_session(&kr, 600, &narrowed),
            Err(TokenError::Expired)
        ));
    }
}
