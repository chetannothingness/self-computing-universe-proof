pub mod hash;
pub mod serpi;
pub mod status;
pub mod receipt;
pub mod provenance;
pub mod tension;
pub mod reason;

pub use hash::H;
pub use serpi::SerPi;
pub use status::Status;
pub use receipt::Receipt;

/// The canonical hash output type: 32 bytes (blake3).
pub type Hash32 = [u8; 32];

/// Zero hash — the genesis / ⊥ value.
pub const HASH_ZERO: Hash32 = [0u8; 32];
