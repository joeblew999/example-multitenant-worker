//! Verified session context inserted into request extensions by the
//! [`crate::middleware::auth::AuthLayer`]. Handlers read it via
//! `ctx.extensions.get::<SessionContext>()`.

use crate::domain::{AuthMethod, BillingAccountId, OrgId, Role, UserId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionContext {
    pub user: UserId,
    pub email: String,
    pub billing: BillingAccountId,
    pub org: Option<OrgId>,
    pub role: Role,
    pub auth_method: AuthMethod,
    pub exp_unix: i64,
}
