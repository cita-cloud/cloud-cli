#[cfg(feature = "sm")]
mod sm;
#[cfg(feature = "sm")]
pub use sm::{generate_keypair, hash_data, pk2address, sign_message};
