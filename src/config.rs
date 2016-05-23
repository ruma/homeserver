//! User-facing configuration.

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use base64::u8de;
use iron::{Plugin, Request};
use iron::typemap::Key;
use persistent::Read as PersistentRead;
use serde_json::from_str;

use error::{APIError, CLIError};

/// Load the user's configuration from a JSON file.
pub fn load(path: &str) -> Result<Config, CLIError> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));

    let config: RawConfig = match from_str(&contents) {
        Ok(config) => config,
        Err(error) => return Err(From::from(error)),
    };

    let macaroon_secret_key = match u8de(config.macaroon_secret_key.as_bytes()) {
        Ok(bytes) => match bytes.len() {
            32 => bytes,
            _ => return Err(CLIError::new("macaroon_secret_key must be 32 bytes.")),
        },
        Err(_) => return Err(CLIError::new(
            "macaroon_secret_key must be valid Base64."
        )),
    };

    Ok(Config {
        bind_address: config.bind_address.unwrap_or("127.0.0.1".to_string()),
        bind_port: config.bind_port.unwrap_or("3000".to_string()),
        domain: config.domain,
        macaroon_secret_key: macaroon_secret_key,
        postgres_url: config.postgres_url,
    })
}

/// Extract the `Config` stored in the request.
pub fn get_config(request: &mut Request) -> Result<Arc<Config>, APIError> {
    request.get::<PersistentRead<Config>>().map_err(APIError::from)
}

/// The user's configuration as loaded from the configuration file.
///
/// Refer to `Config` for the description of the fields.
#[derive(Deserialize)]
struct RawConfig {
    bind_address: Option<String>,
    bind_port: Option<String>,
    domain: String,
    macaroon_secret_key: String,
    postgres_url: String,
}

/// Server configuration provided by the user.
#[derive(Clone)]
pub struct Config {
    /// The network address where the server should listen for connections. Defaults to 127.0.0.1.
    pub bind_address: String,
    /// The network port where the server should listen for connections. Defaults to 3000.
    pub bind_port: String,
    /// The DNS name where clients can reach the server. Used as the hostname portion of user IDs.
    pub domain: String,
    /// The secret key used for generating
    /// [Macaroons](https://research.google.com/pubs/pub41892.html). Must be 32
    /// cryptographically random bytes, encoded as a Base64 string. Changing this value will
    /// invalidate any previously generated macaroons.
    pub macaroon_secret_key: Vec<u8>,
    /// A [PostgreSQL connection string](http://www.postgresql.org/docs/current/static/libpq-connect.html#LIBPQ-CONNSTRING)
    /// for Ruma's PostgreSQL database.
    pub postgres_url: String,
}

impl Key for Config {
    type Value = Config;
}
