pub mod error;
pub mod repo;

#[cfg(not(target_arch = "wasm32"))]
pub mod mem;

#[cfg(target_arch = "wasm32")]
pub mod d1;

pub use error::{StoreError, StoreResult};
pub use repo::{
    BillingAccountWithRole, BillingMemberRow, InvitationAcceptance, NewPasswordUser, NewSsoUser,
    OrgMemberRow, OrgWithRole, Repo,
};

#[cfg(not(target_arch = "wasm32"))]
pub use mem::InMemoryRepo;

#[cfg(target_arch = "wasm32")]
pub use d1::D1Repo;
