use crate::domain::{AuthMethod, Role, SsoConfig, Subscription};
use crate::proto::workers::auth::v1::{
    AuthMethod as AuthMethodPb, AuthMethodKind as AuthMethodKindPb, MembershipSummary,
    Role as RolePb, WhoamiInfo,
};
use crate::proto::workers::billing::v1::{
    SsoConfig as SsoConfigPb, SsoKind as SsoKindPb, Subscription as SubscriptionPb,
    SubscriptionStatus as SubscriptionStatusPb,
};

pub fn role_to_pb(r: Role) -> buffa::EnumValue<RolePb> {
    match r {
        Role::Owner => RolePb::ROLE_OWNER.into(),
        Role::Member => RolePb::ROLE_MEMBER.into(),
    }
}

pub fn auth_method_to_pb(m: &AuthMethod) -> buffa::MessageField<AuthMethodPb> {
    let pb = match m {
        AuthMethod::Password => AuthMethodPb {
            kind: AuthMethodKindPb::AUTH_METHOD_KIND_PASSWORD.into(),
            ..Default::default()
        },
        AuthMethod::Sso(idp_id) => AuthMethodPb {
            kind: AuthMethodKindPb::AUTH_METHOD_KIND_SSO.into(),
            idp_id: idp_id.clone(),
            ..Default::default()
        },
    };
    buffa::MessageField::some(pb)
}

pub fn sso_kind_to_domain(v: i32) -> String {
    match v {
        1 => "oidc".to_owned(),
        2 => "saml".to_owned(),
        _ => format!("unknown:{v}"),
    }
}

pub fn sso_kind_from_domain(s: &str) -> buffa::EnumValue<SsoKindPb> {
    match s {
        "oidc" => SsoKindPb::SSO_KIND_OIDC.into(),
        "saml" => SsoKindPb::SSO_KIND_SAML.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn ms_to_timestamp(ms: i64) -> buffa::MessageField<buffa_types::google::protobuf::Timestamp> {
    let ts = buffa_types::google::protobuf::Timestamp::from_unix(
        ms / 1000,
        ((ms % 1000) * 1_000_000) as i32,
    );
    buffa::MessageField::some(ts)
}

pub fn subscription_status_to_pb(s: &str) -> buffa::EnumValue<SubscriptionStatusPb> {
    match s {
        "active" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_ACTIVE.into(),
        "past_due" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_PAST_DUE.into(),
        "canceled" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_CANCELED.into(),
        "none" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_NONE.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn invoice_status_to_pb(
    s: &str,
) -> buffa::EnumValue<crate::proto::workers::billing::v1::InvoiceStatus> {
    use crate::proto::workers::billing::v1::InvoiceStatus as InvoiceStatusPb;
    match s {
        "paid" => InvoiceStatusPb::INVOICE_STATUS_PAID.into(),
        "open" => InvoiceStatusPb::INVOICE_STATUS_OPEN.into(),
        "void" => InvoiceStatusPb::INVOICE_STATUS_VOID.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn membership_summary(scope_id: String, display_name: String, role: Role) -> MembershipSummary {
    MembershipSummary {
        scope_id,
        display_name,
        role: role_to_pb(role),
        ..Default::default()
    }
}

pub fn subscription_to_pb(s: Subscription) -> SubscriptionPb {
    SubscriptionPb {
        billing_account_id: s.billing_account_id.to_string(),
        plan: s.plan,
        status: subscription_status_to_pb(&s.status),
        payment_method_token: s.payment_method_token,
        updated_at: ms_to_timestamp(s.updated_at_ms),
        ..Default::default()
    }
}

pub fn sso_to_pb(s: SsoConfig) -> SsoConfigPb {
    SsoConfigPb {
        idp_id: s.idp_id,
        kind: sso_kind_from_domain(&s.kind),
        login_url: s.login_url,
        break_glass_user_id: s.break_glass_user_id.to_string(),
        required: s.required,
        ..Default::default()
    }
}

// Re-export WhoamiInfo building here since it's pure conversion logic.
pub fn build_whoami(
    user: &crate::domain::User,
    billing: &crate::domain::BillingAccount,
    org: Option<&crate::domain::Organization>,
    role: Role,
    auth_method: &AuthMethod,
    billings: Vec<crate::store::BillingAccountWithRole>,
    orgs: Vec<crate::store::OrgWithRole>,
) -> WhoamiInfo {
    WhoamiInfo {
        user_id: user.id.to_string(),
        email: user.email.clone(),
        email_verified: user.email_verified,
        billing_account_id: billing.id.to_string(),
        org_id: org.map(|o| o.id.to_string()).unwrap_or_default(),
        role: role_to_pb(role),
        auth_method: auth_method_to_pb(auth_method),
        billing_memberships: billings
            .into_iter()
            .map(|(b, r)| membership_summary(b.id.to_string(), b.display_name, r))
            .collect(),
        org_memberships: orgs
            .into_iter()
            .map(|(o, r)| membership_summary(o.id.to_string(), o.display_name, r))
            .collect(),
        ..Default::default()
    }
}
