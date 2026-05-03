use std::future::Future;

use crate::domain::SsoState;
use crate::store::error::StoreResult;

pub trait AuthFlowRepo: Send + Sync + 'static {
    fn consume_nonce(
        &self,
        nonce: &str,
        purpose: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn store_sso_state(&self, state: SsoState) -> impl Future<Output = StoreResult<()>> + Send;

    fn consume_sso_state(
        &self,
        state: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Option<SsoState>>> + Send;
}
