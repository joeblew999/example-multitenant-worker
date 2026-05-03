use std::collections::HashMap;
use std::future::Future;

use crate::domain::{ScopeKind, SsoConfig};
use crate::store::error::StoreResult;

pub trait SsoConfigRepo: Send + Sync + 'static {
    fn get_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> impl Future<Output = StoreResult<Option<SsoConfig>>> + Send;

    fn list_sso_configs_by_scope_ids(
        &self,
        scope_kind: ScopeKind,
        scope_ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, SsoConfig>>> + Send;

    fn upsert_sso_config(&self, config: SsoConfig) -> impl Future<Output = StoreResult<()>> + Send;

    fn delete_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}
