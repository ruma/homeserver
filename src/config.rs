//! User-facing configuration.

use std::fs::File;
use std::io::Read;

use serde_json::from_str;

use error::CLIError;

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

impl Config {
    /// Load a `Config` from a JSON file.
    pub fn load(path: &str) -> Result<Config, CLIError> {
        let mut file = try!(File::open(path));
        let mut contents = String::new();
        try!(file.read_to_string(&mut contents));

        from_str(&contents).map_err(|error| From::from(error))
    }
}
