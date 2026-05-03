//! In-memory implementation of [`Repo`]. Native unit tests run against
//! this; D1 is wasm-only.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::{
    BillingAccount, BillingAccountId, BillingMembership, Identity, IdentityId, IdentityProvider,
    Invitation, InvitationId, InvitationStatus, OrgId, OrgMembership, Organization, Role,
    ScopeKind, SsoConfig, SsoState, User, UserId, personal_display_name,
};

use super::error::{StoreError, StoreResult};
use super::repo::{
    BillingAccountWithRole, BillingMemberRow, InvitationAcceptance, NewPasswordUser, NewSsoUser,
    OrgMemberRow, OrgWithRole, Repo,
};

#[derive(Default)]
struct State {
    users: HashMap<UserId, User>,
    identities: HashMap<IdentityId, Identity>,
    billing_accounts: HashMap<BillingAccountId, BillingAccount>,
    organizations: HashMap<OrgId, Organization>,
    billing_memberships: HashMap<(UserId, BillingAccountId), BillingMembership>,
    org_memberships: HashMap<(UserId, OrgId), OrgMembership>,
    sso_configs: HashMap<(ScopeKind, String), SsoConfig>,
    invitations: HashMap<InvitationId, Invitation>,
    consumed_nonces: HashMap<String, (String, i64)>,
    sso_states: HashMap<String, SsoState>,
}

#[derive(Default)]
pub struct InMemoryRepo {
    inner: Mutex<State>,
}

impl InMemoryRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Repo for InMemoryRepo {
    async fn get_user(&self, id: &UserId) -> StoreResult<Option<User>> {
        let g = self.inner.lock().unwrap();
        Ok(g.users.get(id).cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> StoreResult<Option<User>> {
        let g = self.inner.lock().unwrap();
        Ok(g.users
            .values()
            .find(|u| u.email.eq_ignore_ascii_case(email))
            .cloned())
    }

    async fn mark_email_verified(&self, user_id: &UserId) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let user = g
            .users
            .get_mut(user_id)
            .ok_or_else(|| StoreError::not_found(format!("user {user_id}")))?;
        user.email_verified = true;
        Ok(())
    }

    async fn list_identities(&self, user_id: &UserId) -> StoreResult<Vec<Identity>> {
        let g = self.inner.lock().unwrap();
        Ok(g.identities
            .values()
            .filter(|i| i.user_id == *user_id)
            .cloned()
            .collect())
    }

    async fn find_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> StoreResult<Option<Identity>> {
        let g = self.inner.lock().unwrap();
        Ok(g.identities
            .values()
            .find(|i| i.provider == provider && i.provider_user_id == provider_user_id)
            .cloned())
    }

    async fn add_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> StoreResult<Identity> {
        let mut g = self.inner.lock().unwrap();
        if g.identities
            .values()
            .any(|i| i.provider == provider && i.provider_user_id == provider_user_id)
        {
            return Err(StoreError::already_exists(format!(
                "identity {provider}:{provider_user_id}"
            )));
        }
        let id = IdentityId::new();
        let identity = Identity {
            id: id.clone(),
            user_id: user_id.clone(),
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            secret,
            created_at_ms: now_ms,
        };
        g.identities.insert(id, identity.clone());
        Ok(identity)
    }

    async fn update_identity_secret(
        &self,
        identity_id: &IdentityId,
        new_secret: String,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let identity = g
            .identities
            .get_mut(identity_id)
            .ok_or_else(|| StoreError::not_found(format!("identity {identity_id}")))?;
        identity.secret = Some(new_secret);
        Ok(())
    }

    async fn delete_user(&self, user_id: &UserId) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        if !g.users.contains_key(user_id) {
            return Err(StoreError::not_found(format!("user {user_id}")));
        }
        g.identities.retain(|_, i| i.user_id != *user_id);
        g.billing_memberships.retain(|(u, _), _| u != user_id);
        g.org_memberships.retain(|(u, _), _| u != user_id);
        let personal_orgs: Vec<OrgId> = g
            .organizations
            .values()
            .filter(|o| o.personal && o.owner_user_id.as_ref() == Some(user_id))
            .map(|o| o.id.clone())
            .collect();
        for po in personal_orgs {
            g.organizations.remove(&po);
        }
        let personal_billing: Vec<BillingAccountId> = g
            .billing_accounts
            .values()
            .filter(|b| b.personal && b.owner_user_id.as_ref() == Some(user_id))
            .map(|b| b.id.clone())
            .collect();
        for pb in personal_billing {
            g.billing_accounts.remove(&pb);
        }
        // Matches the SQL schema's ON DELETE SET NULL on inviter_user_id.
        for inv in g.invitations.values_mut() {
            if inv.inviter_user_id.as_ref() == Some(user_id) {
                inv.inviter_user_id = None;
            }
        }
        g.users.remove(user_id);
        Ok(())
    }

    async fn create_password_user(
        &self,
        input: NewPasswordUser,
        invite: Option<InvitationAcceptance>,
        now_ms: i64,
    ) -> StoreResult<User> {
        let mut g = self.inner.lock().unwrap();
        if g.users
            .values()
            .any(|u| u.email.eq_ignore_ascii_case(&input.email))
        {
            return Err(StoreError::already_exists(format!(
                "user with email {}",
                input.email
            )));
        }
        let user_id = UserId::new();
        let user = User {
            id: user_id.clone(),
            email: input.email.clone(),
            email_verified: false,
            created_at_ms: now_ms,
        };
        g.users.insert(user_id.clone(), user.clone());

        let identity_id = IdentityId::new();
        g.identities.insert(
            identity_id.clone(),
            Identity {
                id: identity_id,
                user_id: user_id.clone(),
                provider: IdentityProvider::PASSWORD.into(),
                provider_user_id: user_id.to_string(),
                secret: Some(input.password_hash),
                created_at_ms: now_ms,
            },
        );

        let billing_id = BillingAccountId::new();
        g.billing_accounts.insert(
            billing_id.clone(),
            BillingAccount {
                id: billing_id.clone(),
                display_name: personal_display_name(&input.email),
                personal: true,
                owner_user_id: Some(user_id.clone()),
                auto_join_domain: None,
                created_at_ms: now_ms,
            },
        );
        g.billing_memberships.insert(
            (user_id.clone(), billing_id.clone()),
            BillingMembership {
                user_id: user_id.clone(),
                billing_account_id: billing_id.clone(),
                role: Role::Owner,
                created_at_ms: now_ms,
            },
        );

        insert_personal_org_under_lock(&mut g, &user_id, &input.email, &billing_id, now_ms);

        if let Some(invite) = invite {
            apply_invitation_under_lock(&mut g, &user_id, invite, now_ms)?;
        }
        Ok(user)
    }

    async fn create_sso_user(
        &self,
        input: NewSsoUser,
        invite: Option<InvitationAcceptance>,
        auto_join_billing: Option<BillingAccountId>,
        now_ms: i64,
    ) -> StoreResult<User> {
        let mut g = self.inner.lock().unwrap();
        if g.users
            .values()
            .any(|u| u.email.eq_ignore_ascii_case(&input.email))
        {
            return Err(StoreError::already_exists(format!(
                "user with email {}",
                input.email
            )));
        }
        let user_id = UserId::new();
        let user = User {
            id: user_id.clone(),
            email: input.email.clone(),
            // SSO assertions are taken at face value for the demo; the IdP
            // already verified the email.
            email_verified: true,
            created_at_ms: now_ms,
        };
        g.users.insert(user_id.clone(), user.clone());

        let provider = IdentityProvider::sso_str(&input.idp_id);
        let identity_id = IdentityId::new();
        g.identities.insert(
            identity_id.clone(),
            Identity {
                id: identity_id,
                user_id: user_id.clone(),
                provider,
                provider_user_id: input.idp_user_id,
                secret: None,
                created_at_ms: now_ms,
            },
        );

        let billing_id = BillingAccountId::new();
        g.billing_accounts.insert(
            billing_id.clone(),
            BillingAccount {
                id: billing_id.clone(),
                display_name: personal_display_name(&input.email),
                personal: true,
                owner_user_id: Some(user_id.clone()),
                auto_join_domain: None,
                created_at_ms: now_ms,
            },
        );
        g.billing_memberships.insert(
            (user_id.clone(), billing_id.clone()),
            BillingMembership {
                user_id: user_id.clone(),
                billing_account_id: billing_id.clone(),
                role: Role::Owner,
                created_at_ms: now_ms,
            },
        );

        insert_personal_org_under_lock(&mut g, &user_id, &input.email, &billing_id, now_ms);

        if let Some(invite) = invite {
            apply_invitation_under_lock(&mut g, &user_id, invite, now_ms)?;
        }

        if let Some(target) = auto_join_billing
            && g.billing_accounts.contains_key(&target)
        {
            g.billing_memberships.insert(
                (user_id.clone(), target.clone()),
                BillingMembership {
                    user_id: user_id.clone(),
                    billing_account_id: target,
                    role: Role::Member,
                    created_at_ms: now_ms,
                },
            );
        }

        Ok(user)
    }

    async fn link_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> StoreResult<Identity> {
        let mut g = self.inner.lock().unwrap();
        if let Some(existing) = g
            .identities
            .values()
            .find(|i| i.provider == provider && i.provider_user_id == provider_user_id)
            .cloned()
        {
            if existing.user_id != *user_id {
                return Err(StoreError::conflict(format!(
                    "identity {provider}:{provider_user_id} belongs to another user"
                )));
            }
            return Ok(existing);
        }
        let id = IdentityId::new();
        let identity = Identity {
            id: id.clone(),
            user_id: user_id.clone(),
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            secret,
            created_at_ms: now_ms,
        };
        g.identities.insert(id, identity.clone());
        Ok(identity)
    }

    async fn get_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> StoreResult<Option<BillingAccount>> {
        let g = self.inner.lock().unwrap();
        Ok(g.billing_accounts.get(id).cloned())
    }

    async fn list_billing_accounts_for_user(
        &self,
        user_id: &UserId,
    ) -> StoreResult<Vec<BillingAccountWithRole>> {
        let g = self.inner.lock().unwrap();
        let mut out: Vec<BillingAccountWithRole> = g
            .billing_memberships
            .iter()
            .filter(|((u, _), _)| u == user_id)
            .filter_map(|((_, ba), m)| g.billing_accounts.get(ba).cloned().map(|b| (b, m.role)))
            .collect();
        out.sort_by(|(a, _), (b, _)| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    async fn create_billing_account(
        &self,
        display_name: String,
        owner_user_id: UserId,
        now_ms: i64,
    ) -> StoreResult<BillingAccount> {
        let mut g = self.inner.lock().unwrap();
        if !g.users.contains_key(&owner_user_id) {
            return Err(StoreError::not_found(format!("user {owner_user_id}")));
        }
        let id = BillingAccountId::new();
        let account = BillingAccount {
            id: id.clone(),
            display_name,
            personal: false,
            owner_user_id: None,
            auto_join_domain: None,
            created_at_ms: now_ms,
        };
        g.billing_accounts.insert(id.clone(), account.clone());
        g.billing_memberships.insert(
            (owner_user_id.clone(), id.clone()),
            BillingMembership {
                user_id: owner_user_id,
                billing_account_id: id,
                role: Role::Owner,
                created_at_ms: now_ms,
            },
        );
        Ok(account)
    }

    async fn set_auto_join_domain(
        &self,
        billing_account_id: &BillingAccountId,
        domain: Option<String>,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let account = g
            .billing_accounts
            .get_mut(billing_account_id)
            .ok_or_else(|| {
                StoreError::not_found(format!("billing account {billing_account_id}"))
            })?;
        account.auto_join_domain = domain.map(|d| d.to_lowercase());
        Ok(())
    }

    async fn find_billing_by_auto_join_domain(
        &self,
        domain: &str,
    ) -> StoreResult<Option<BillingAccount>> {
        let g = self.inner.lock().unwrap();
        Ok(g.billing_accounts
            .values()
            .find(|b| {
                !b.personal
                    && b.auto_join_domain
                        .as_deref()
                        .is_some_and(|d| d.eq_ignore_ascii_case(domain))
            })
            .cloned())
    }

    async fn delete_billing_account(&self, id: &BillingAccountId) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let account = g
            .billing_accounts
            .get(id)
            .cloned()
            .ok_or_else(|| StoreError::not_found(format!("billing account {id}")))?;
        if account.personal {
            return Err(StoreError::conflict(
                "personal billing accounts are deleted with the user",
            ));
        }
        if g.organizations
            .values()
            .any(|o| o.billing_account_id == *id)
        {
            return Err(StoreError::conflict(
                "cannot delete billing account: orgs still attached",
            ));
        }
        g.billing_accounts.remove(id);
        g.billing_memberships.retain(|(_, b), _| b != id);
        g.sso_configs
            .retain(|(kind, sid), _| !(*kind == ScopeKind::Billing && sid == id.as_str()));
        Ok(())
    }

    async fn get_billing_accounts_by_ids(
        &self,
        ids: &[&str],
    ) -> StoreResult<HashMap<String, BillingAccount>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let g = self.inner.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| {
                g.billing_accounts
                    .get(&BillingAccountId::from(*id))
                    .map(|b| ((*id).to_owned(), b.clone()))
            })
            .collect())
    }

    async fn count_orgs_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<i64> {
        let g = self.inner.lock().unwrap();
        Ok(g.organizations
            .values()
            .filter(|o| o.billing_account_id == *billing_account_id)
            .count() as i64)
    }

    async fn count_billing_owners(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<i64> {
        let g = self.inner.lock().unwrap();
        Ok(g.billing_memberships
            .values()
            .filter(|m| m.billing_account_id == *billing_account_id && m.role == Role::Owner)
            .count() as i64)
    }

    async fn count_org_owners(&self, org_id: &OrgId) -> StoreResult<i64> {
        let g = self.inner.lock().unwrap();
        Ok(g.org_memberships
            .values()
            .filter(|m| m.org_id == *org_id && m.role == Role::Owner)
            .count() as i64)
    }

    async fn count_non_personal_owner_memberships(&self, user_id: &UserId) -> StoreResult<i64> {
        let g = self.inner.lock().unwrap();
        Ok(g.billing_memberships
            .values()
            .filter(|m| m.user_id == *user_id && m.role == Role::Owner)
            .filter(|m| {
                g.billing_accounts
                    .get(&m.billing_account_id)
                    .map(|b| !b.personal)
                    .unwrap_or(false)
            })
            .count() as i64)
    }

    async fn create_organization(
        &self,
        display_name: String,
        billing_account_id: BillingAccountId,
        now_ms: i64,
    ) -> StoreResult<Organization> {
        let mut g = self.inner.lock().unwrap();
        if !g.billing_accounts.contains_key(&billing_account_id) {
            return Err(StoreError::not_found(format!(
                "billing account {billing_account_id}"
            )));
        }
        let id = OrgId::new();
        let org = Organization {
            id: id.clone(),
            display_name,
            billing_account_id,
            personal: false,
            owner_user_id: None,
            created_at_ms: now_ms,
        };
        g.organizations.insert(id, org.clone());
        Ok(org)
    }

    async fn get_organization(&self, id: &OrgId) -> StoreResult<Option<Organization>> {
        let g = self.inner.lock().unwrap();
        Ok(g.organizations.get(id).cloned())
    }

    async fn list_organizations_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Vec<Organization>> {
        let g = self.inner.lock().unwrap();
        let mut out: Vec<Organization> = g
            .organizations
            .values()
            .filter(|o| o.billing_account_id == *billing_account_id)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    async fn list_organizations_for_user(&self, user_id: &UserId) -> StoreResult<Vec<OrgWithRole>> {
        let g = self.inner.lock().unwrap();
        let mut out: Vec<OrgWithRole> = g
            .org_memberships
            .iter()
            .filter(|((u, _), _)| u == user_id)
            .filter_map(|((_, oid), m)| g.organizations.get(oid).cloned().map(|o| (o, m.role)))
            .collect();
        out.sort_by(|(a, _), (b, _)| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    async fn delete_organization(&self, id: &OrgId) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let org = g
            .organizations
            .get(id)
            .cloned()
            .ok_or_else(|| StoreError::not_found(format!("organization {id}")))?;
        if org.personal {
            return Err(StoreError::conflict(
                "personal organizations are deleted with the user",
            ));
        }
        g.organizations.remove(id);
        g.org_memberships.retain(|(_, oid), _| oid != id);
        g.sso_configs
            .retain(|(kind, sid), _| !(*kind == ScopeKind::Org && sid == id.as_str()));
        Ok(())
    }

    async fn get_organizations_by_ids(
        &self,
        ids: &[&str],
    ) -> StoreResult<HashMap<String, Organization>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let g = self.inner.lock().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| {
                g.organizations
                    .get(&OrgId::from(*id))
                    .map(|o| ((*id).to_owned(), o.clone()))
            })
            .collect())
    }

    async fn add_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
        now_ms: i64,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), billing_account_id.clone());
        if g.billing_memberships.contains_key(&key) {
            return Err(StoreError::already_exists(format!(
                "billing membership {user_id} -> {billing_account_id}"
            )));
        }
        g.billing_memberships.insert(
            key,
            BillingMembership {
                user_id: user_id.clone(),
                billing_account_id: billing_account_id.clone(),
                role,
                created_at_ms: now_ms,
            },
        );
        Ok(())
    }

    async fn get_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Option<BillingMembership>> {
        let g = self.inner.lock().unwrap();
        Ok(g.billing_memberships
            .get(&(user_id.clone(), billing_account_id.clone()))
            .cloned())
    }

    async fn update_billing_membership_role(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), billing_account_id.clone());
        let existing_role = g
            .billing_memberships
            .get(&key)
            .map(|m| m.role)
            .ok_or_else(|| StoreError::not_found("billing membership"))?;
        if existing_role == Role::Owner && role != Role::Owner {
            let owner_count = g
                .billing_memberships
                .values()
                .filter(|x| x.billing_account_id == *billing_account_id && x.role == Role::Owner)
                .count();
            if owner_count <= 1 {
                return Err(StoreError::conflict(
                    "cannot demote the last owner of a billing account",
                ));
            }
        }
        if let Some(mem) = g.billing_memberships.get_mut(&key) {
            mem.role = role;
        }
        Ok(())
    }

    async fn remove_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), billing_account_id.clone());
        let existing = g
            .billing_memberships
            .get(&key)
            .cloned()
            .ok_or_else(|| StoreError::not_found("billing membership"))?;
        if existing.role == Role::Owner {
            let owner_count = g
                .billing_memberships
                .values()
                .filter(|x| x.billing_account_id == *billing_account_id && x.role == Role::Owner)
                .count();
            if owner_count <= 1 {
                return Err(StoreError::conflict(
                    "cannot remove the last owner of a billing account",
                ));
            }
        }
        g.billing_memberships.remove(&key);
        Ok(())
    }

    async fn list_billing_memberships(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Vec<BillingMemberRow>> {
        let g = self.inner.lock().unwrap();
        let mut out: Vec<BillingMemberRow> = g
            .billing_memberships
            .iter()
            .filter(|(_, m)| m.billing_account_id == *billing_account_id)
            .filter_map(|((uid, _), m)| g.users.get(uid).cloned().map(|u| (m.clone(), u)))
            .collect();
        out.sort_by(|(a, _), (b, _)| a.user_id.as_str().cmp(b.user_id.as_str()));
        Ok(out)
    }

    async fn add_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
        now_ms: i64,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), org_id.clone());
        if g.org_memberships.contains_key(&key) {
            return Err(StoreError::already_exists(format!(
                "org membership {user_id} -> {org_id}"
            )));
        }
        g.org_memberships.insert(
            key,
            OrgMembership {
                user_id: user_id.clone(),
                org_id: org_id.clone(),
                role,
                created_at_ms: now_ms,
            },
        );
        Ok(())
    }

    async fn get_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> StoreResult<Option<OrgMembership>> {
        let g = self.inner.lock().unwrap();
        Ok(g.org_memberships
            .get(&(user_id.clone(), org_id.clone()))
            .cloned())
    }

    async fn update_org_membership_role(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), org_id.clone());
        let existing_role = g
            .org_memberships
            .get(&key)
            .map(|m| m.role)
            .ok_or_else(|| StoreError::not_found("org membership"))?;
        if existing_role == Role::Owner && role != Role::Owner {
            let owner_count = g
                .org_memberships
                .values()
                .filter(|x| x.org_id == *org_id && x.role == Role::Owner)
                .count();
            if owner_count <= 1 {
                return Err(StoreError::conflict(
                    "cannot demote the last owner of an organization",
                ));
            }
        }
        if let Some(mem) = g.org_memberships.get_mut(&key) {
            mem.role = role;
        }
        Ok(())
    }

    async fn remove_org_membership(&self, user_id: &UserId, org_id: &OrgId) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let key = (user_id.clone(), org_id.clone());
        let existing = g
            .org_memberships
            .get(&key)
            .cloned()
            .ok_or_else(|| StoreError::not_found("org membership"))?;
        if existing.role == Role::Owner {
            let owner_count = g
                .org_memberships
                .values()
                .filter(|x| x.org_id == *org_id && x.role == Role::Owner)
                .count();
            if owner_count <= 1 {
                return Err(StoreError::conflict(
                    "cannot remove the last owner of an organization",
                ));
            }
        }
        g.org_memberships.remove(&key);
        Ok(())
    }

    async fn list_org_memberships(&self, org_id: &OrgId) -> StoreResult<Vec<OrgMemberRow>> {
        let g = self.inner.lock().unwrap();
        let mut out: Vec<OrgMemberRow> = g
            .org_memberships
            .iter()
            .filter(|(_, m)| m.org_id == *org_id)
            .filter_map(|((uid, _), m)| g.users.get(uid).cloned().map(|u| (m.clone(), u)))
            .collect();
        out.sort_by(|(a, _), (b, _)| a.user_id.as_str().cmp(b.user_id.as_str()));
        Ok(out)
    }

    async fn get_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> StoreResult<Option<SsoConfig>> {
        let g = self.inner.lock().unwrap();
        Ok(g.sso_configs
            .get(&(scope_kind, scope_id.to_owned()))
            .cloned())
    }

    async fn list_sso_configs_by_scope_ids(
        &self,
        scope_kind: ScopeKind,
        scope_ids: &[&str],
    ) -> StoreResult<HashMap<String, SsoConfig>> {
        if scope_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let g = self.inner.lock().unwrap();
        Ok(scope_ids
            .iter()
            .filter_map(|id| {
                g.sso_configs
                    .get(&(scope_kind, (*id).to_owned()))
                    .map(|c| ((*id).to_owned(), c.clone()))
            })
            .collect())
    }

    async fn upsert_sso_config(&self, config: SsoConfig) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        g.sso_configs
            .insert((config.scope_kind, config.scope_id.clone()), config);
        Ok(())
    }

    async fn delete_sso_config(&self, scope_kind: ScopeKind, scope_id: &str) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        g.sso_configs.remove(&(scope_kind, scope_id.to_owned()));
        Ok(())
    }

    async fn create_invitation(&self, invitation: Invitation) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        // Re-inviting the same address must go through refresh_invitation_token
        // rather than stacking pending rows.
        if g.invitations.values().any(|i| {
            i.status == InvitationStatus::Pending
                && i.scope_kind == invitation.scope_kind
                && i.scope_id == invitation.scope_id
                && i.email.eq_ignore_ascii_case(&invitation.email)
        }) {
            return Err(StoreError::already_exists(format!(
                "pending invite for {} on {}:{}",
                invitation.email,
                invitation.scope_kind.as_str(),
                invitation.scope_id
            )));
        }
        g.invitations.insert(invitation.id.clone(), invitation);
        Ok(())
    }

    async fn get_invitation(&self, id: &InvitationId) -> StoreResult<Option<Invitation>> {
        let g = self.inner.lock().unwrap();
        Ok(g.invitations.get(id).cloned())
    }

    async fn list_pending_invitations_by_email(&self, email: &str) -> StoreResult<Vec<Invitation>> {
        let g = self.inner.lock().unwrap();
        Ok(g.invitations
            .values()
            .filter(|i| {
                i.status == InvitationStatus::Pending && i.email.eq_ignore_ascii_case(email)
            })
            .cloned()
            .collect())
    }

    async fn list_pending_invitation_by_scope_email(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
        email: &str,
    ) -> StoreResult<Option<Invitation>> {
        let g = self.inner.lock().unwrap();
        Ok(g.invitations
            .values()
            .find(|i| {
                i.status == InvitationStatus::Pending
                    && i.scope_kind == scope_kind
                    && i.scope_id == scope_id
                    && i.email.eq_ignore_ascii_case(email)
            })
            .cloned())
    }

    async fn update_invitation_status(
        &self,
        id: &InvitationId,
        status: InvitationStatus,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let inv = g
            .invitations
            .get_mut(id)
            .ok_or_else(|| StoreError::not_found(format!("invitation {id}")))?;
        inv.status = status;
        Ok(())
    }

    async fn refresh_invitation_token(
        &self,
        id: &InvitationId,
        new_nonce: String,
        new_expiry_ms: i64,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        let inv = g
            .invitations
            .get_mut(id)
            .ok_or_else(|| StoreError::not_found(format!("invitation {id}")))?;
        inv.nonce = new_nonce;
        inv.expires_at_ms = new_expiry_ms;
        inv.status = InvitationStatus::Pending;
        Ok(())
    }

    async fn accept_invitation_existing_user(
        &self,
        user_id: &UserId,
        acceptance: InvitationAcceptance,
        nonce: &str,
        now_ms: i64,
    ) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        if g.consumed_nonces.contains_key(nonce) {
            return Err(StoreError::already_exists(format!("nonce {nonce}")));
        }
        // Stamp the nonce first so a concurrent caller can't double-spend.
        g.consumed_nonces
            .insert(nonce.to_owned(), ("invitation".into(), now_ms));
        apply_invitation_under_lock(&mut g, user_id, acceptance, now_ms)?;
        Ok(())
    }

    async fn consume_nonce(&self, nonce: &str, purpose: &str, now_ms: i64) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        if g.consumed_nonces.contains_key(nonce) {
            return Err(StoreError::already_exists(format!("nonce {nonce}")));
        }
        g.consumed_nonces
            .insert(nonce.to_owned(), (purpose.to_owned(), now_ms));
        Ok(())
    }

    async fn store_sso_state(&self, state: SsoState) -> StoreResult<()> {
        let mut g = self.inner.lock().unwrap();
        g.sso_states.insert(state.state.clone(), state);
        Ok(())
    }

    async fn consume_sso_state(&self, state: &str, now_ms: i64) -> StoreResult<Option<SsoState>> {
        let mut g = self.inner.lock().unwrap();
        let Some(s) = g.sso_states.remove(state) else {
            return Ok(None);
        };
        if s.expires_at_ms < now_ms {
            return Ok(None);
        }
        Ok(Some(s))
    }
}

fn insert_personal_org_under_lock(
    g: &mut State,
    user_id: &UserId,
    email: &str,
    billing_id: &BillingAccountId,
    now_ms: i64,
) {
    let org_id = OrgId::new();
    g.organizations.insert(
        org_id.clone(),
        Organization {
            id: org_id.clone(),
            display_name: personal_display_name(email),
            billing_account_id: billing_id.clone(),
            personal: true,
            owner_user_id: Some(user_id.clone()),
            created_at_ms: now_ms,
        },
    );
    g.org_memberships.insert(
        (user_id.clone(), org_id.clone()),
        OrgMembership {
            user_id: user_id.clone(),
            org_id,
            role: Role::Owner,
            created_at_ms: now_ms,
        },
    );
}

fn apply_invitation_under_lock(
    g: &mut State,
    user_id: &UserId,
    acceptance: InvitationAcceptance,
    now_ms: i64,
) -> StoreResult<()> {
    match acceptance.scope_kind {
        ScopeKind::Billing => {
            let billing_id = BillingAccountId::from_string(acceptance.scope_id);
            if !g.billing_accounts.contains_key(&billing_id) {
                return Err(StoreError::not_found(format!(
                    "billing account {billing_id}"
                )));
            }
            g.billing_memberships.insert(
                (user_id.clone(), billing_id.clone()),
                BillingMembership {
                    user_id: user_id.clone(),
                    billing_account_id: billing_id,
                    role: acceptance.role,
                    created_at_ms: now_ms,
                },
            );
        }
        ScopeKind::Org => {
            let org_id = OrgId::from_string(acceptance.scope_id);
            if !g.organizations.contains_key(&org_id) {
                return Err(StoreError::not_found(format!("organization {org_id}")));
            }
            g.org_memberships.insert(
                (user_id.clone(), org_id.clone()),
                OrgMembership {
                    user_id: user_id.clone(),
                    org_id,
                    role: acceptance.role,
                    created_at_ms: now_ms,
                },
            );
        }
    }
    if let Some(inv) = g.invitations.get_mut(&acceptance.invitation_id) {
        inv.status = InvitationStatus::Accepted;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn signup_creates_user_identity_and_personal_billing_and_org() {
        let repo = InMemoryRepo::new();
        let user = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "ada@example.com".into(),
                password_hash: "hash".into(),
            },
            None,
            1_000,
        ))
        .unwrap();
        assert_eq!(user.email, "ada@example.com");
        let billing = block_on(repo.list_billing_accounts_for_user(&user.id)).unwrap();
        assert_eq!(billing.len(), 1);
        assert!(billing[0].0.personal);
        assert_eq!(billing[0].1, Role::Owner);
        let identities = block_on(repo.list_identities(&user.id)).unwrap();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities[0].provider, "password");
        let orgs = block_on(repo.list_organizations_for_user(&user.id)).unwrap();
        assert_eq!(orgs.len(), 1);
        assert!(orgs[0].0.personal);
        assert_eq!(orgs[0].0.owner_user_id.as_ref(), Some(&user.id));
        assert_eq!(orgs[0].0.billing_account_id, billing[0].0.id);
        assert_eq!(orgs[0].1, Role::Owner);
    }

    #[test]
    fn duplicate_signup_rejected() {
        let repo = InMemoryRepo::new();
        let _ = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "x@y".into(),
                password_hash: "h".into(),
            },
            None,
            1,
        ))
        .unwrap();
        let err = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "X@Y".into(), // case-insensitive
                password_hash: "h".into(),
            },
            None,
            1,
        ))
        .unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists(_)));
    }

    #[test]
    fn delete_billing_account_refuses_personal() {
        let repo = InMemoryRepo::new();
        let user = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "x@y".into(),
                password_hash: "h".into(),
            },
            None,
            1,
        ))
        .unwrap();
        let billings = block_on(repo.list_billing_accounts_for_user(&user.id)).unwrap();
        let err = block_on(repo.delete_billing_account(&billings[0].0.id)).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[test]
    fn delete_billing_account_refuses_non_personal_with_orgs() {
        let repo = InMemoryRepo::new();
        let user = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "x@y".into(),
                password_hash: "h".into(),
            },
            None,
            1,
        ))
        .unwrap();
        let billing =
            block_on(repo.create_billing_account("Acme".into(), user.id.clone(), 2)).unwrap();
        block_on(repo.create_organization("Eng".into(), billing.id.clone(), 3)).unwrap();
        let err = block_on(repo.delete_billing_account(&billing.id)).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[test]
    fn cannot_remove_last_billing_owner() {
        let repo = InMemoryRepo::new();
        let user = block_on(repo.create_password_user(
            NewPasswordUser {
                email: "x@y".into(),
                password_hash: "h".into(),
            },
            None,
            1,
        ))
        .unwrap();
        let billing =
            block_on(repo.create_billing_account("Acme".into(), user.id.clone(), 2)).unwrap();
        let err = block_on(repo.remove_billing_membership(&user.id, &billing.id)).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[test]
    fn nonce_replay_rejected() {
        let repo = InMemoryRepo::new();
        block_on(repo.consume_nonce("n", "test", 1)).unwrap();
        let err = block_on(repo.consume_nonce("n", "test", 2)).unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists(_)));
    }
}
