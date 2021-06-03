#[cfg(feature = "crypto_sm")]
mod sm;
#[cfg(feature = "crypto_sm")]
pub use sm::{generate_keypair, hash_data, pk2address, sign_message};

#[cfg(feature = "crypto_eth")]
mod eth;
#[cfg(feature = "crypto_eth")]
pub use eth::{generate_keypair, hash_data, pk2address, sign_message};
