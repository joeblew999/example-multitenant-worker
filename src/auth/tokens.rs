//! Mint / verify the four kinds of macaroon this service issues.
//!
//! All tokens carry:
//!   - `purpose=<session|invitation|password_reset|email_verify>`
//!   - `exp=<unix_seconds>`
//!
//! Session tokens additionally carry user / scope / role / auth_method
//! caveats. One-shot tokens carry a `nonce` consumed via `Repo::consume_nonce`
//! at the use RPC.
//!
//! Verification is pure (no DB reads). The use RPC stamps the nonce.

use libmacaroon::{Caveat, Format, Macaroon, MacaroonError, Verifier};

use crate::domain::{
    AuthMethod, BillingAccountId, InvitationId, OrgId, Role, ScopeKind, TokenPurpose, UserId,
};

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
    MissingCaveat(&'static str),
    InvalidCaveat(String),
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
            TokenError::MissingCaveat(c) => write!(f, "missing caveat: {c}"),
            TokenError::InvalidCaveat(c) => write!(f, "invalid caveat: {c}"),
        }
    }
}

impl std::error::Error for TokenError {}

impl From<MacaroonError> for TokenError {
    fn from(e: MacaroonError) -> Self {
        match e {
            MacaroonError::InvalidSignature => TokenError::InvalidSignature,
            MacaroonError::CaveatNotSatisfied(s) => TokenError::InvalidCaveat(s),
            MacaroonError::DeserializationError(s) => TokenError::Malformed(s),
            other => TokenError::Malformed(other.to_string()),
        }
    }
}

fn cv(prefix: &str, value: impl AsRef<str>) -> String {
    format!("{prefix}={}", value.as_ref())
}

fn parse_caveat<'a>(c: &'a Caveat, prefix: &str) -> Option<&'a str> {
    match c {
        Caveat::FirstParty(fp) => {
            let p = fp.predicate();
            std::str::from_utf8(p)
                .ok()
                .and_then(|s| s.strip_prefix(prefix))
        }
        Caveat::ThirdParty(_) => None,
    }
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
    let exp = input.now_unix + input.ttl_seconds;
    let mut mac = Macaroon::create(
        Some("workers-multitenant"),
        keyring.root(),
        format!("session:{}", input.user_id),
    )?;
    mac.add_first_party_caveat(cv("purpose", TokenPurpose::Session.as_str()))?
        .add_first_party_caveat(cv("user", input.user_id.as_str()))?
        .add_first_party_caveat(cv("email", input.email))?
        .add_first_party_caveat(cv("billing", input.billing.as_str()))?
        .add_first_party_caveat(cv("role", input.role.as_str()))?
        .add_first_party_caveat(cv("auth_method", input.auth_method.encode()))?
        .add_first_party_caveat(cv("exp", exp.to_string()))?;
    if let Some(org) = input.org {
        mac.add_first_party_caveat(cv("org", org.as_str()))?;
    }
    Ok(mac.serialize(Format::V2)?)
}

pub fn verify_session(
    keyring: &Keyring,
    now_unix: i64,
    token: &str,
) -> Result<SessionContext, TokenError> {
    let macaroon = Macaroon::deserialize(token)?;
    let mut verifier = Verifier::default();
    register_session_satisfiers(&mut verifier, now_unix);
    verifier.verify(&macaroon, keyring.root(), &[])?;

    let mut user: Option<UserId> = None;
    let mut email: Option<String> = None;
    let mut billing: Option<BillingAccountId> = None;
    let mut org: Option<OrgId> = None;
    let mut role: Option<Role> = None;
    let mut auth_method: Option<AuthMethod> = None;
    let mut exp_unix: Option<i64> = None;
    let mut purpose: Option<String> = None;

    for c in macaroon.caveats() {
        if let Some(v) = parse_caveat(c, "purpose=") {
            purpose = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "user=") {
            user = Some(UserId::from(v));
        } else if let Some(v) = parse_caveat(c, "email=") {
            email = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "billing=") {
            billing = Some(BillingAccountId::from(v));
        } else if let Some(v) = parse_caveat(c, "org=") {
            org = Some(OrgId::from(v));
        } else if let Some(v) = parse_caveat(c, "role=") {
            role = v.parse().map(Some).map_err(TokenError::InvalidCaveat)?;
        } else if let Some(v) = parse_caveat(c, "auth_method=") {
            auth_method = Some(
                AuthMethod::decode(v)
                    .ok_or_else(|| TokenError::InvalidCaveat(format!("auth_method={v}")))?,
            );
        } else if let Some(v) = parse_caveat(c, "exp=") {
            exp_unix = Some(
                v.parse()
                    .map_err(|_| TokenError::InvalidCaveat(format!("exp={v}")))?,
            );
        }
    }

    let purpose = purpose.ok_or(TokenError::MissingCaveat("purpose"))?;
    if purpose != TokenPurpose::Session.as_str() {
        return Err(TokenError::WrongPurpose {
            expected: TokenPurpose::Session,
            found: purpose,
        });
    }

    Ok(SessionContext {
        user: user.ok_or(TokenError::MissingCaveat("user"))?,
        email: email.ok_or(TokenError::MissingCaveat("email"))?,
        billing: billing.ok_or(TokenError::MissingCaveat("billing"))?,
        org,
        role: role.ok_or(TokenError::MissingCaveat("role"))?,
        auth_method: auth_method.ok_or(TokenError::MissingCaveat("auth_method"))?,
        exp_unix: exp_unix.ok_or(TokenError::MissingCaveat("exp"))?,
    })
}

fn register_session_satisfiers(verifier: &mut Verifier, now_unix: i64) {
    verifier.satisfy_exact(cv("purpose", TokenPurpose::Session.as_str()));
    verifier.satisfy_general(prefix_satisfier(b"user="));
    verifier.satisfy_general(prefix_satisfier(b"email="));
    verifier.satisfy_general(prefix_satisfier(b"billing="));
    verifier.satisfy_general(prefix_satisfier(b"org="));
    verifier.satisfy_general(prefix_satisfier(b"role="));
    verifier.satisfy_general(prefix_satisfier(b"auth_method="));
    verifier.satisfy_general(move |c: &[u8]| exp_satisfier(c, now_unix));
}

fn prefix_satisfier(prefix: &'static [u8]) -> impl Fn(&[u8]) -> bool + Send + Sync + 'static {
    move |c: &[u8]| c.starts_with(prefix)
}

fn exp_satisfier(caveat: &[u8], now_unix: i64) -> bool {
    let Some(rest) = caveat.strip_prefix(b"exp=") else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return false;
    };
    let Ok(when) = text.parse::<i64>() else {
        return false;
    };
    when > now_unix
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
    let exp = input.now_unix + input.ttl_seconds;
    let mut mac = Macaroon::create(
        Some("workers-multitenant"),
        keyring.root(),
        format!("invitation:{}", input.invitation_id),
    )?;
    mac.add_first_party_caveat(cv("purpose", TokenPurpose::Invitation.as_str()))?
        .add_first_party_caveat(cv("invitation_id", input.invitation_id.as_str()))?
        .add_first_party_caveat(cv("email", input.email))?
        .add_first_party_caveat(cv("scope_kind", input.scope_kind.as_str()))?
        .add_first_party_caveat(cv("scope_id", input.scope_id))?
        .add_first_party_caveat(cv("role", input.role.as_str()))?
        .add_first_party_caveat(cv("nonce", input.nonce))?
        .add_first_party_caveat(cv("exp", exp.to_string()))?;
    if let Some(idp) = input.required_idp {
        mac.add_first_party_caveat(cv("required_idp", idp))?;
    }
    Ok(mac.serialize(Format::V2)?)
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
    let macaroon = Macaroon::deserialize(token)?;
    let mut verifier = Verifier::default();
    verifier.satisfy_exact(cv("purpose", TokenPurpose::Invitation.as_str()));
    for prefix in [
        &b"invitation_id="[..],
        &b"email="[..],
        &b"scope_kind="[..],
        &b"scope_id="[..],
        &b"role="[..],
        &b"nonce="[..],
        &b"required_idp="[..],
    ] {
        verifier.satisfy_general(prefix_satisfier(prefix));
    }
    verifier.satisfy_general(move |c: &[u8]| exp_satisfier(c, now_unix));
    verifier.verify(&macaroon, keyring.root(), &[])?;

    let mut invitation_id: Option<InvitationId> = None;
    let mut email: Option<String> = None;
    let mut scope_kind: Option<ScopeKind> = None;
    let mut scope_id: Option<String> = None;
    let mut role: Option<Role> = None;
    let mut required_idp: Option<String> = None;
    let mut nonce: Option<String> = None;
    let mut exp_unix: Option<i64> = None;
    let mut purpose: Option<String> = None;

    for c in macaroon.caveats() {
        if let Some(v) = parse_caveat(c, "purpose=") {
            purpose = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "invitation_id=") {
            invitation_id = Some(InvitationId::from(v));
        } else if let Some(v) = parse_caveat(c, "email=") {
            email = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "scope_kind=") {
            scope_kind = Some(v.parse().map_err(TokenError::InvalidCaveat)?);
        } else if let Some(v) = parse_caveat(c, "scope_id=") {
            scope_id = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "role=") {
            role = Some(v.parse().map_err(TokenError::InvalidCaveat)?);
        } else if let Some(v) = parse_caveat(c, "required_idp=") {
            required_idp = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "nonce=") {
            nonce = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "exp=") {
            exp_unix = Some(
                v.parse()
                    .map_err(|_| TokenError::InvalidCaveat(format!("exp={v}")))?,
            );
        }
    }

    let purpose = purpose.ok_or(TokenError::MissingCaveat("purpose"))?;
    if purpose != TokenPurpose::Invitation.as_str() {
        return Err(TokenError::WrongPurpose {
            expected: TokenPurpose::Invitation,
            found: purpose,
        });
    }

    Ok(InvitationToken {
        invitation_id: invitation_id.ok_or(TokenError::MissingCaveat("invitation_id"))?,
        email: email.ok_or(TokenError::MissingCaveat("email"))?,
        scope_kind: scope_kind.ok_or(TokenError::MissingCaveat("scope_kind"))?,
        scope_id: scope_id.ok_or(TokenError::MissingCaveat("scope_id"))?,
        role: role.ok_or(TokenError::MissingCaveat("role"))?,
        required_idp,
        nonce: nonce.ok_or(TokenError::MissingCaveat("nonce"))?,
        exp_unix: exp_unix.ok_or(TokenError::MissingCaveat("exp"))?,
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
    let exp = now_unix + ttl_seconds;
    let mut mac = Macaroon::create(
        Some("workers-multitenant"),
        keyring.root(),
        format!("{}:{user_id}", purpose.as_str()),
    )?;
    mac.add_first_party_caveat(cv("purpose", purpose.as_str()))?
        .add_first_party_caveat(cv("user", user_id.as_str()))?
        .add_first_party_caveat(cv("nonce", nonce))?
        .add_first_party_caveat(cv("exp", exp.to_string()))?;
    Ok(mac.serialize(Format::V2)?)
}

fn verify_one_shot(
    keyring: &Keyring,
    expected: TokenPurpose,
    now_unix: i64,
    token: &str,
) -> Result<OneShotFields, TokenError> {
    let macaroon = Macaroon::deserialize(token)?;
    let mut verifier = Verifier::default();
    verifier.satisfy_exact(cv("purpose", expected.as_str()));
    verifier.satisfy_general(prefix_satisfier(b"user="));
    verifier.satisfy_general(prefix_satisfier(b"nonce="));
    verifier.satisfy_general(move |c: &[u8]| exp_satisfier(c, now_unix));
    verifier.verify(&macaroon, keyring.root(), &[])?;

    let mut user_id: Option<UserId> = None;
    let mut nonce: Option<String> = None;
    let mut exp_unix: Option<i64> = None;
    let mut purpose: Option<String> = None;

    for c in macaroon.caveats() {
        if let Some(v) = parse_caveat(c, "purpose=") {
            purpose = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "user=") {
            user_id = Some(UserId::from(v));
        } else if let Some(v) = parse_caveat(c, "nonce=") {
            nonce = Some(v.into());
        } else if let Some(v) = parse_caveat(c, "exp=") {
            exp_unix = Some(
                v.parse()
                    .map_err(|_| TokenError::InvalidCaveat(format!("exp={v}")))?,
            );
        }
    }
    let purpose = purpose.ok_or(TokenError::MissingCaveat("purpose"))?;
    if purpose != expected.as_str() {
        return Err(TokenError::WrongPurpose {
            expected,
            found: purpose,
        });
    }
    Ok(OneShotFields {
        user_id: user_id.ok_or(TokenError::MissingCaveat("user"))?,
        nonce: nonce.ok_or(TokenError::MissingCaveat("nonce"))?,
        exp_unix: exp_unix.ok_or(TokenError::MissingCaveat("exp"))?,
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
        let user = UserId::from("u_2");
        let billing = BillingAccountId::from("ba_2");
        let token = mint_session(
            &kr,
            MintSessionInput {
                user_id: &user,
                email: "x@y",
                billing: &billing,
                org: None,
                role: Role::Member,
                auth_method: &AuthMethod::Password,
                now_unix: 0,
                ttl_seconds: 10,
            },
        )
        .unwrap();
        assert!(verify_session(&kr, 100, &token).is_err());
    }

    #[test]
    fn session_rejects_wrong_key() {
        let kr = keyring();
        let other = Keyring::from_key(libmacaroon::MacaroonKey::generate(b"other"));
        let user = UserId::from("u_3");
        let billing = BillingAccountId::from("ba_3");
        let token = mint_session(
            &kr,
            MintSessionInput {
                user_id: &user,
                email: "x@y",
                billing: &billing,
                org: None,
                role: Role::Owner,
                auth_method: &AuthMethod::Password,
                now_unix: 0,
                ttl_seconds: 60,
            },
        )
        .unwrap();
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
        let user = UserId::from("u_x");
        let session_token = mint_session(
            &kr,
            MintSessionInput {
                user_id: &user,
                email: "x@y",
                billing: &BillingAccountId::from("ba_x"),
                org: None,
                role: Role::Member,
                auth_method: &AuthMethod::Password,
                now_unix: 0,
                ttl_seconds: 60,
            },
        )
        .unwrap();
        // Treat the session token as an invitation — must fail.
        assert!(verify_invitation(&kr, 1, &session_token).is_err());
    }
}
