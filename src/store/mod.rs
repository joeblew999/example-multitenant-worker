pub mod error;
pub mod repo;

pub mod mem;

#[cfg(target_arch = "wasm32")]
pub mod d1;

pub use error::{StoreError, StoreResult};
pub use mem::InMemoryRepo;
pub use repo::{
    BillingAccountWithRole, BillingMemberRow, InvitationAcceptance, NewPasswordUser, NewSsoUser,
    OrgMemberRow, OrgWithRole, Repo,
};

#[cfg(target_arch = "wasm32")]
pub use d1::D1Repo;
