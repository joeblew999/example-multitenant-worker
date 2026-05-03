use std::future::Future;

use crate::domain::{
    BillingAccountId, BillingMembership, OrgId, OrgMembership, Role, User, UserId,
};
use crate::store::error::StoreResult;

pub type BillingMemberRow = (BillingMembership, User);
pub type OrgMemberRow = (OrgMembership, User);

pub trait MembershipRepo: Send + Sync + 'static {
    fn add_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Option<BillingMembership>>> + Send;

    fn update_billing_membership_role(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn remove_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn list_billing_memberships(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Vec<BillingMemberRow>>> + Send;

    fn add_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<Option<OrgMembership>>> + Send;

    fn update_org_membership_role(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn remove_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn list_org_memberships(
        &self,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<Vec<OrgMemberRow>>> + Send;

    fn count_org_owners(&self, org_id: &OrgId) -> impl Future<Output = StoreResult<i64>> + Send;

    fn count_non_personal_owner_memberships(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;
}
