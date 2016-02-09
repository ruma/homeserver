//! User-facing configuration.

use std::fs::File;
use std::io::Read;

use serde_json::from_str;

use error::CLIError;

/// Load the user's configuration from a JSON file.
pub fn load(path: &str) -> Result<FinalConfig, CLIError> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));

    let config: Config = match from_str(&contents) {
        Ok(config) => config,
        Err(error) => return Err(From::from(error)),
    };

    Ok(FinalConfig {
        bind_address: config.bind_address.unwrap_or("127.0.0.1".to_owned()),
        bind_port: config.bind_port.unwrap_or("3000".to_owned()),
        domain: config.domain,
        postgres_url: config.postgres_url,
    })
}

/// The user's configuration.
#[derive(Deserialize)]
pub struct Config {
    /// The network address where the server should listen for connections. Defaults to 127.0.0.1.
    pub bind_address: Option<String>,
    /// The network port where the server should listen for connections. Defaults to 3000.
    pub bind_port: Option<String>,
    /// The DNS name where clients can reach the server. Used as the hostname portion of user IDs.
    pub domain: String,
    /// A [PostgreSQL connection string](http://www.postgresql.org/docs/current/static/libpq-connect.html#LIBPQ-CONNSTRING) for Ruma's PostgreSQL database.
    pub postgres_url: String,
}

/// The user's configuration with defaults for missing fields filled in.
///
/// Refer to `Config` for the description of the fields.
pub struct FinalConfig {
    pub bind_address: String,
    pub bind_port: String,
    pub domain: String,
    pub postgres_url: String,
}
