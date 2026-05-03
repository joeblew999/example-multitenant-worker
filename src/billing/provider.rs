//! Seam for swapping the demo's storage-backed billing for a real
//! provider (Stripe, Lago, ...). A real adapter is a third impl of
//! `BillingProvider`.

use std::future::Future;

use crate::domain::{BillingAccountId, Invoice, InvoiceId, Subscription};
use crate::store::error::{StoreError, StoreResult};

pub trait BillingProvider: Send + Sync + 'static {
    fn get_subscription(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Subscription>> + Send;

    fn set_payment_method(
        &self,
        billing_account_id: &BillingAccountId,
        payment_method_token: String,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Subscription>> + Send;

    fn cancel_subscription(
        &self,
        billing_account_id: &BillingAccountId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Subscription>> + Send;

    fn list_invoices(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Vec<Invoice>>> + Send;

    fn record_invoice(
        &self,
        billing_account_id: &BillingAccountId,
        amount_cents: i64,
        currency: String,
        status: String,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Invoice>> + Send;

    /// Idempotent: inserts a default-state subscription row if none exists.
    fn ensure_subscription(
        &self,
        billing_account_id: &BillingAccountId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::InMemoryBillingProvider;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct InMemoryBillingProvider {
        subscriptions: Mutex<HashMap<BillingAccountId, Subscription>>,
        invoices: Mutex<HashMap<BillingAccountId, Vec<Invoice>>>,
    }

    impl InMemoryBillingProvider {
        pub fn new() -> Self {
            Self::default()
        }
    }

    fn default_subscription(billing_account_id: &BillingAccountId, now_ms: i64) -> Subscription {
        Subscription {
            billing_account_id: billing_account_id.clone(),
            plan: "free".into(),
            status: "none".into(),
            payment_method_token: None,
            updated_at_ms: now_ms,
        }
    }

    impl BillingProvider for InMemoryBillingProvider {
        async fn get_subscription(
            &self,
            billing_account_id: &BillingAccountId,
        ) -> StoreResult<Subscription> {
            let g = self.subscriptions.lock().unwrap();
            g.get(billing_account_id)
                .cloned()
                .ok_or_else(|| StoreError::not_found(format!("subscription {billing_account_id}")))
        }

        async fn set_payment_method(
            &self,
            billing_account_id: &BillingAccountId,
            payment_method_token: String,
            now_ms: i64,
        ) -> StoreResult<Subscription> {
            let sub = Subscription {
                billing_account_id: billing_account_id.clone(),
                plan: "starter".into(),
                status: "active".into(),
                payment_method_token: Some(payment_method_token),
                updated_at_ms: now_ms,
            };
            self.subscriptions
                .lock()
                .unwrap()
                .insert(billing_account_id.clone(), sub.clone());
            Ok(sub)
        }

        async fn cancel_subscription(
            &self,
            billing_account_id: &BillingAccountId,
            now_ms: i64,
        ) -> StoreResult<Subscription> {
            let mut g = self.subscriptions.lock().unwrap();
            let sub = g.get_mut(billing_account_id).ok_or_else(|| {
                StoreError::not_found(format!("subscription {billing_account_id}"))
            })?;
            sub.status = "canceled".into();
            sub.updated_at_ms = now_ms;
            Ok(sub.clone())
        }

        async fn list_invoices(
            &self,
            billing_account_id: &BillingAccountId,
        ) -> StoreResult<Vec<Invoice>> {
            let g = self.invoices.lock().unwrap();
            Ok(g.get(billing_account_id).cloned().unwrap_or_default())
        }

        async fn record_invoice(
            &self,
            billing_account_id: &BillingAccountId,
            amount_cents: i64,
            currency: String,
            status: String,
            now_ms: i64,
        ) -> StoreResult<Invoice> {
            let inv = Invoice {
                id: InvoiceId::new(),
                billing_account_id: billing_account_id.clone(),
                amount_cents,
                currency,
                status,
                issued_at_ms: now_ms,
            };
            let mut g = self.invoices.lock().unwrap();
            g.entry(billing_account_id.clone())
                .or_default()
                .push(inv.clone());
            Ok(inv)
        }

        async fn ensure_subscription(
            &self,
            billing_account_id: &BillingAccountId,
            now_ms: i64,
        ) -> StoreResult<()> {
            let mut g = self.subscriptions.lock().unwrap();
            g.entry(billing_account_id.clone())
                .or_insert_with(|| default_subscription(billing_account_id, now_ms));
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::D1BillingProvider;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use serde::Deserialize;
    use worker::send::IntoSendFuture;
    use worker::{D1Database, D1Type};

    pub struct D1BillingProvider {
        db: D1Database,
    }

    impl D1BillingProvider {
        pub fn new(db: D1Database) -> Self {
            Self { db }
        }
    }

    fn backend(s: impl Into<String>) -> StoreError {
        StoreError::backend(s.into())
    }
    fn ms(x: i64) -> D1Type<'static> {
        D1Type::Real(x as f64)
    }

    #[derive(Deserialize)]
    struct SubRow {
        billing_account_id: String,
        plan: String,
        status: String,
        payment_method_token: Option<String>,
        updated_at_ms: i64,
    }
    impl From<SubRow> for Subscription {
        fn from(r: SubRow) -> Self {
            Subscription {
                billing_account_id: BillingAccountId::from_string(r.billing_account_id),
                plan: r.plan,
                status: r.status,
                payment_method_token: r.payment_method_token,
                updated_at_ms: r.updated_at_ms,
            }
        }
    }

    #[derive(Deserialize)]
    struct InvoiceRow {
        id: String,
        billing_account_id: String,
        amount_cents: i64,
        currency: String,
        status: String,
        issued_at_ms: i64,
    }
    impl From<InvoiceRow> for Invoice {
        fn from(r: InvoiceRow) -> Self {
            Invoice {
                id: InvoiceId::from_string(r.id),
                billing_account_id: BillingAccountId::from_string(r.billing_account_id),
                amount_cents: r.amount_cents,
                currency: r.currency,
                status: r.status,
                issued_at_ms: r.issued_at_ms,
            }
        }
    }

    impl BillingProvider for D1BillingProvider {
        async fn get_subscription(
            &self,
            billing_account_id: &BillingAccountId,
        ) -> StoreResult<Subscription> {
            let row: Option<SubRow> = self
                .db
                .prepare(
                    "SELECT billing_account_id, plan, status, payment_method_token, updated_at_ms \
                     FROM subscriptions WHERE billing_account_id = ?",
                )
                .bind_refs(&[D1Type::Text(billing_account_id.as_str())])
                .map_err(|e| backend(format!("get_subscription bind: {e}")))?
                .first(None)
                .into_send()
                .await
                .map_err(|e| backend(format!("get_subscription: {e}")))?;
            row.map(Into::into)
                .ok_or_else(|| StoreError::not_found(format!("subscription {billing_account_id}")))
        }

        async fn set_payment_method(
            &self,
            billing_account_id: &BillingAccountId,
            payment_method_token: String,
            now_ms: i64,
        ) -> StoreResult<Subscription> {
            self.db
                .prepare(
                    "INSERT INTO subscriptions (billing_account_id, plan, status, payment_method_token, updated_at_ms) \
                     VALUES (?, 'starter', 'active', ?, ?) \
                     ON CONFLICT(billing_account_id) DO UPDATE SET \
                       plan = 'starter', status = 'active', \
                       payment_method_token = excluded.payment_method_token, \
                       updated_at_ms = excluded.updated_at_ms",
                )
                .bind_refs(&[
                    D1Type::Text(billing_account_id.as_str()),
                    D1Type::Text(&payment_method_token),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("set_payment_method bind: {e}")))?
                .run()
                .into_send()
                .await
                .map_err(|e| backend(format!("set_payment_method: {e}")))?;
            Ok(Subscription {
                billing_account_id: billing_account_id.clone(),
                plan: "starter".into(),
                status: "active".into(),
                payment_method_token: Some(payment_method_token),
                updated_at_ms: now_ms,
            })
        }

        async fn cancel_subscription(
            &self,
            billing_account_id: &BillingAccountId,
            now_ms: i64,
        ) -> StoreResult<Subscription> {
            let result = self
                .db
                .prepare("UPDATE subscriptions SET status = 'canceled', updated_at_ms = ? WHERE billing_account_id = ?")
                .bind_refs(&[ms(now_ms), D1Type::Text(billing_account_id.as_str())])
                .map_err(|e| backend(format!("cancel_subscription bind: {e}")))?
                .run()
                .into_send()
                .await
                .map_err(|e| backend(format!("cancel_subscription: {e}")))?;
            let changes = result
                .meta()
                .map_err(|e| backend(format!("meta: {e}")))?
                .and_then(|m| m.changes)
                .unwrap_or(0);
            if changes == 0 {
                return Err(StoreError::not_found(format!(
                    "subscription {billing_account_id}"
                )));
            }
            self.get_subscription(billing_account_id).await
        }

        async fn list_invoices(
            &self,
            billing_account_id: &BillingAccountId,
        ) -> StoreResult<Vec<Invoice>> {
            let rows: Vec<InvoiceRow> = self
                .db
                .prepare(
                    "SELECT id, billing_account_id, amount_cents, currency, status, issued_at_ms \
                     FROM invoices WHERE billing_account_id = ? ORDER BY issued_at_ms DESC",
                )
                .bind_refs(&[D1Type::Text(billing_account_id.as_str())])
                .map_err(|e| backend(format!("list_invoices bind: {e}")))?
                .all()
                .into_send()
                .await
                .map_err(|e| backend(format!("list_invoices: {e}")))?
                .results()
                .map_err(|e| backend(format!("list_invoices results: {e}")))?;
            Ok(rows.into_iter().map(Into::into).collect())
        }

        async fn record_invoice(
            &self,
            billing_account_id: &BillingAccountId,
            amount_cents: i64,
            currency: String,
            status: String,
            now_ms: i64,
        ) -> StoreResult<Invoice> {
            let id = InvoiceId::new();
            let id_str = id.to_string();
            self.db
                .prepare(
                    "INSERT INTO invoices (id, billing_account_id, amount_cents, currency, status, issued_at_ms) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(&id_str),
                    D1Type::Text(billing_account_id.as_str()),
                    D1Type::Real(amount_cents as f64),
                    D1Type::Text(&currency),
                    D1Type::Text(&status),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("record_invoice bind: {e}")))?
                .run()
                .into_send()
                .await
                .map_err(|e| backend(format!("record_invoice: {e}")))?;
            Ok(Invoice {
                id,
                billing_account_id: billing_account_id.clone(),
                amount_cents,
                currency,
                status,
                issued_at_ms: now_ms,
            })
        }

        async fn ensure_subscription(
            &self,
            billing_account_id: &BillingAccountId,
            now_ms: i64,
        ) -> StoreResult<()> {
            self.db
                .prepare(
                    "INSERT OR IGNORE INTO subscriptions (billing_account_id, plan, status, payment_method_token, updated_at_ms) \
                     VALUES (?, 'free', 'none', NULL, ?)",
                )
                .bind_refs(&[D1Type::Text(billing_account_id.as_str()), ms(now_ms)])
                .map_err(|e| backend(format!("ensure_subscription bind: {e}")))?
                .run()
                .into_send()
                .await
                .map_err(|e| backend(format!("ensure_subscription: {e}")))?;
            Ok(())
        }
    }
}
