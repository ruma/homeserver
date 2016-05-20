//! Cryptographic operations.

use argon2rs::{Argon2, Variant};
use argon2rs::verifier::Verifier;
use base64::u8en;
use rand::{OsRng, Rng};

use error::{APIError, CLIError};

/// Generates a random 32-byte secret key for macaroons.
pub fn generate_macaroon_secret_key() -> Result<String, CLIError> {
    let mut rng = try!(OsRng::new());
    let mut key = [0u8; 32];

    rng.fill_bytes(&mut key);

    let encoded = try!(u8en(&key).map_err(CLIError::from));
    let encoded_string = try!(String::from_utf8(encoded));

    Ok(encoded_string)
}

/// Hash a password with Argon2.
pub fn hash_password(password: &str) -> Result<String, APIError> {
    let salt = try!(generate_salt());
    let argon2 = Argon2::default(Variant::Argon2i);
    let verifier = Verifier::new(argon2, password.as_bytes(), &salt, &[], &[]);
    let encoded_hash = verifier.to_u8();

    String::from_utf8(encoded_hash).map_err(APIError::from)
}

/// Verifies a password with Argon2.
pub fn verify_password(encoded_hash: &[u8], plaintext_password: &str)
-> Result<bool, APIError> {
    let verifier = match Verifier::from_u8(encoded_hash) {
        Ok(verifier) => verifier,
        Err(error) => {
            let message = format!("argon2rs verifier error: {:?}", error);

            return Err(APIError::unknown_from_string(message));
        }
    };

    Ok(verifier.verify(plaintext_password.as_bytes()))
}

/// Generates a random salt for Argon2.
fn generate_salt() -> Result<[u8; 16], APIError> {
    let mut rng = try!(OsRng::new());
    let mut salt = [0u8; 16];

    rng.fill_bytes(&mut salt);

    Ok(salt)
}
