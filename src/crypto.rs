//! Cryptographic operations.

use argon2rs::verifier::Encoded;
use base64::encode;
use rand::{
    RngCore,
    rngs::OsRng,
};

use error::{ApiError, CliError};

/// Generates a random 32-byte secret key for macaroons.
pub fn generate_macaroon_secret_key() -> Result<String, CliError> {
    let mut rng = OsRng::new()?;
    let mut key = [0u8; 32];

    rng.fill_bytes(&mut key);

    Ok(encode(&key))
}

/// Hash a password with Argon2.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = generate_salt()?;
    let encoded_hash = Encoded::default2i(password.as_bytes(), &salt, &[], &[]).to_u8();

    String::from_utf8(encoded_hash).map_err(ApiError::from)
}

/// Verifies a password with Argon2.
pub fn verify_password(encoded_hash: &[u8], plaintext_password: &str)
-> Result<bool, ApiError> {
    let encoded = Encoded::from_u8(encoded_hash).map_err(ApiError::from)?;

    Ok(encoded.verify(plaintext_password.as_bytes()))
}

/// Generates a random salt for Argon2.
fn generate_salt() -> Result<[u8; 16], ApiError> {
    let mut rng = OsRng::new()?;
    let mut salt = [0u8; 16];

    rng.fill_bytes(&mut salt);

    Ok(salt)
}
