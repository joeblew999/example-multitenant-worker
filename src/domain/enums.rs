//! Enumerated domain values. We use string constants on the wire (proto3
//! doesn't carry enum richness across language boundaries cleanly) and
//! parse into typed enums at the service-handler boundary.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Member,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Member => "member",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Role::Owner),
            "member" => Ok(Role::Member),
            other => Err(format!("unknown role: {other}")),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeKind {
    Billing,
    Org,
}

impl ScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScopeKind::Billing => "billing",
            ScopeKind::Org => "org",
        }
    }
}

impl fmt::Display for ScopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ScopeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "billing" => Ok(ScopeKind::Billing),
            "org" => Ok(ScopeKind::Org),
            other => Err(format!("unknown scope kind: {other}")),
        }
    }
}

/// How the current session was authenticated. Encoded as a caveat
/// (`auth_method=password` or `auth_method=sso:<idp_id>`) so that scope-entry
/// time can compare against an SSO-required scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    Password,
    Sso(String),
}

impl AuthMethod {
    pub fn encode(&self) -> String {
        match self {
            AuthMethod::Password => "password".into(),
            AuthMethod::Sso(idp) => format!("sso:{idp}"),
        }
    }

    pub fn decode(s: &str) -> Option<Self> {
        if s == "password" {
            Some(AuthMethod::Password)
        } else {
            s.strip_prefix("sso:")
                .map(|idp| AuthMethod::Sso(idp.to_owned()))
        }
    }
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// Identity provider tag stored on `identities.provider`. `password` is
/// special-cased; everything else is OAuth/SSO.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IdentityProvider {
    Password,
    GitHub,
    Google,
    Sso(String),
}

impl IdentityProvider {
    pub const PASSWORD: &'static str = "password";
    pub const GITHUB: &'static str = "github";
    pub const GOOGLE: &'static str = "google";

    pub fn encode(&self) -> String {
        match self {
            IdentityProvider::Password => Self::PASSWORD.into(),
            IdentityProvider::GitHub => Self::GITHUB.into(),
            IdentityProvider::Google => Self::GOOGLE.into(),
            IdentityProvider::Sso(idp) => Self::sso_str(idp),
        }
    }

    pub fn decode(s: &str) -> Option<Self> {
        match s {
            Self::PASSWORD => Some(IdentityProvider::Password),
            Self::GITHUB => Some(IdentityProvider::GitHub),
            Self::GOOGLE => Some(IdentityProvider::Google),
            other => other
                .strip_prefix("sso:")
                .map(|idp| IdentityProvider::Sso(idp.to_owned())),
        }
    }

    /// Encode an SSO provider tag without allocating an enum value. Use
    /// when you have the IdP id as a borrowed string and just need the
    /// `sso:<idp>` wire format.
    pub fn sso_str(idp_id: &str) -> String {
        format!("sso:{idp_id}")
    }
}

impl fmt::Display for IdentityProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeTarget {
    Billing(super::ids::BillingAccountId),
    Org(super::ids::OrgId),
}

impl ScopeTarget {
    pub fn scope_kind(&self) -> ScopeKind {
        match self {
            ScopeTarget::Billing(_) => ScopeKind::Billing,
            ScopeTarget::Org(_) => ScopeKind::Org,
        }
    }

    pub fn scope_id_str(&self) -> &str {
        match self {
            ScopeTarget::Billing(id) => id.as_str(),
            ScopeTarget::Org(id) => id.as_str(),
        }
    }

    pub fn into_scope_id_string(self) -> String {
        match self {
            ScopeTarget::Billing(id) => id.into_string(),
            ScopeTarget::Org(id) => id.into_string(),
        }
    }

    pub fn from_parts(kind: ScopeKind, id: String) -> Self {
        match kind {
            ScopeKind::Billing => ScopeTarget::Billing(super::ids::BillingAccountId::from(id)),
            ScopeKind::Org => ScopeTarget::Org(super::ids::OrgId::from(id)),
        }
    }
}

/// `purpose=` caveat values for the four kinds of macaroon this service
/// mints. Verification rejects a token whose purpose doesn't match.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TokenPurpose {
    Session,
    Invitation,
    PasswordReset,
    EmailVerify,
}

impl TokenPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenPurpose::Session => "session",
            TokenPurpose::Invitation => "invitation",
            TokenPurpose::PasswordReset => "password_reset",
            TokenPurpose::EmailVerify => "email_verify",
        }
    }
}

impl fmt::Display for TokenPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_roundtrips() {
        assert_eq!(Role::Owner.as_str().parse::<Role>().unwrap(), Role::Owner);
        assert_eq!(Role::Member.as_str().parse::<Role>().unwrap(), Role::Member);
    }

    #[test]
    fn auth_method_roundtrips() {
        assert_eq!(
            AuthMethod::decode("password").unwrap(),
            AuthMethod::Password
        );
        assert_eq!(
            AuthMethod::decode("sso:acme-okta").unwrap(),
            AuthMethod::Sso("acme-okta".into()),
        );
        assert_eq!(AuthMethod::Password.encode(), "password");
        assert_eq!(AuthMethod::Sso("x".into()).encode(), "sso:x");
        assert!(AuthMethod::decode("magic").is_none());
    }
}
