use std::collections::HashMap;
use std::future::Future;

use crate::domain::{BillingAccountId, OrgId, Organization, Role, UserId};
use crate::store::error::StoreResult;

pub type OrgWithRole = (Organization, Role);

pub trait OrgRepo: Send + Sync + 'static {
    fn create_organization(
        &self,
        display_name: String,
        billing_account_id: BillingAccountId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Organization>> + Send;

    fn get_organization(
        &self,
        id: &OrgId,
    ) -> impl Future<Output = StoreResult<Option<Organization>>> + Send;

    fn list_organizations_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Vec<Organization>>> + Send;

    fn list_organizations_for_user(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<Vec<OrgWithRole>>> + Send;

    fn delete_organization(&self, id: &OrgId) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_organizations_by_ids(
        &self,
        ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, Organization>>> + Send;
}
