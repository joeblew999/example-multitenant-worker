mod auth_flow;
mod billing;
mod invitation;
mod membership;
mod org;
mod sso_config;
mod user;

pub use auth_flow::AuthFlowRepo;
pub use billing::{BillingAccountWithRole, BillingRepo};
pub use invitation::{InvitationAcceptance, InvitationRepo};
pub use membership::{BillingMemberRow, MembershipRepo, OrgMemberRow};
pub use org::{OrgRepo, OrgWithRole};
pub use sso_config::SsoConfigRepo;
pub use user::{NewPasswordUser, NewSsoUser, UserRepo};

pub trait Repo:
    UserRepo + BillingRepo + OrgRepo + MembershipRepo + SsoConfigRepo + InvitationRepo + AuthFlowRepo
{
}

impl<T> Repo for T where
    T: UserRepo
        + BillingRepo
        + OrgRepo
        + MembershipRepo
        + SsoConfigRepo
        + InvitationRepo
        + AuthFlowRepo
{
}
