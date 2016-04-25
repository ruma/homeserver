//! Cryptographic operations.

use argon2rs::{Argon2, Variant};
use argon2rs::defaults::LENGTH;
use base64::u8en;
use rand::{OsRng, Rng};

use error::{APIError, CLIError};

/// Hash a password with Argon2.
pub fn hash_password(password: &str) -> Result<String, APIError> {
    let salt = try!(generate_salt());
    let argon2 = Argon2::default(Variant::Argon2i);
    let mut hash = [0; LENGTH];

    argon2.hash(&mut hash, password.as_bytes(), &salt, &[], &[]);

    let encoded = try!(u8en(&hash).map_err(APIError::from));

    String::from_utf8(encoded).map_err(APIError::from)
}

/// Generates a random 32-byte secret key for macaroons.
pub fn generate_macaroon_secret_key() -> Result<String, CLIError> {
    let mut rng = try!(OsRng::new());
    let mut key = [0u8; 32];

    rng.fill_bytes(&mut key);

    let encoded = try!(u8en(&key).map_err(CLIError::from));
    let encoded_string = try!(String::from_utf8(encoded));

    Ok(encoded_string)
}

/// Generates a random salt for Argon2.
fn generate_salt() -> Result<[u8; 16], APIError> {
    let mut rng = try!(OsRng::new());
    let mut salt = [0u8; 16];

    rng.fill_bytes(&mut salt);

    Ok(salt)
}
