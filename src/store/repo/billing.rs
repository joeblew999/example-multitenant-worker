use std::collections::HashMap;
use std::future::Future;

use crate::domain::{BillingAccount, BillingAccountId, Role, UserId};
use crate::store::error::StoreResult;

pub type BillingAccountWithRole = (BillingAccount, Role);

pub trait BillingRepo: Send + Sync + 'static {
    fn get_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Option<BillingAccount>>> + Send;

    fn list_billing_accounts_for_user(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<Vec<BillingAccountWithRole>>> + Send;

    fn create_billing_account(
        &self,
        display_name: String,
        owner_user_id: UserId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<BillingAccount>> + Send;

    fn set_auto_join_domain(
        &self,
        billing_account_id: &BillingAccountId,
        domain: Option<String>,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn find_billing_by_auto_join_domain(
        &self,
        domain: &str,
    ) -> impl Future<Output = StoreResult<Option<BillingAccount>>> + Send;

    fn delete_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_billing_accounts_by_ids(
        &self,
        ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, BillingAccount>>> + Send;

    fn count_orgs_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;

    fn count_billing_owners(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;
}
