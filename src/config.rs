//! User-facing configuration.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use base64::decode;
use iron::{Plugin, Request};
use iron::typemap::Key;
use persistent::Read as PersistentRead;
use serde_json;
use serde_yaml;
use toml;

use error::{ApiError, CliError};

/// The user's configuration as loaded from the configuration file.
///
/// Refer to `Config` for the description of the fields.
#[derive(Deserialize, RustcDecodable)]
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

impl Config {
    /// Load the user's configuration file.
    pub fn from_file() -> Result<Config, CliError> {
        let config: RawConfig;

        if Self::json_exists() {
            config = Self::load_json()?;
        } else if Self::toml_exists() {
            config = Self::load_toml()?;
        } else if Self::yaml_exists() {
            config = Self::load_yaml()?;
        } else {
            return Err(CliError::new("No configuration file was found."));
        }

        let macaroon_secret_key = match decode(&config.macaroon_secret_key) {
            Ok(bytes) => match bytes.len() {
                32 => bytes,
                _ => Err(CliError::new("macaroon_secret_key must be 32 bytes."))?,
            },
            Err(_) => Err(CliError::new("macaroon_secret_key must be valid Base64."))?,
        };

        Ok(Config {
            bind_address: config.bind_address.unwrap_or("127.0.0.1".to_string()),
            bind_port: config.bind_port.unwrap_or("3000".to_string()),
            domain: config.domain,
            macaroon_secret_key: macaroon_secret_key,
            postgres_url: config.postgres_url,
        })
    }

    /// Load the `RawConfig` from a JSON configuration file.
    fn load_json() -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents("ruma.json");
        match serde_json::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Load the `RawConfig` from a TOML configuration file.
    fn load_toml() -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents("ruma.toml");
        let mut parser = toml::Parser::new(&contents);
        let data  = parser.parse();

        if data.is_none() {
            for err in &parser.errors {
                let (loline, locol) = parser.to_linecol(err.lo);
                let (hiline, hicol) = parser.to_linecol(err.hi);
                println!("ruma.toml: {}:{}-{}:{} error: {}", loline, locol, hiline, hicol, err.desc);
            }

            return Err(CliError::new("Unable to parse ruma.toml."));
        }

        let config = toml::Value::Table(data.unwrap());
        match toml::decode(config) {
            Some(t) => Ok(t),
            None => Err(CliError::new("Error while decoding ruma.toml.")),
        }
    }

    /// Load the `RawConfig` from a YAML configuration file.
    fn load_yaml() -> Result<RawConfig, CliError> {
        let contents = if Path::new("ruma.yaml").is_file() {
            Self::read_file_contents("ruma.yaml")
        } else {
            Self::read_file_contents("ruma.yml")
        };

        match serde_yaml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Read the contents of a file.
    fn read_file_contents(path: &str) -> String {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    /// Check if there is a configuration file in JSON.
    fn json_exists() -> bool {
        Path::new("ruma.json").is_file()
    }

    /// Check if there is a configuration file in TOML.
    fn toml_exists() -> bool {
        Path::new("ruma.toml").is_file()
    }

    /// Check if there is a configuration file in YAML.
    fn yaml_exists() -> bool {
        Path::new("ruma.yml").is_file() || Path::new("ruma.yaml").is_file()
    }

    /// Extract the `Config` stored in the request.
    pub fn from_request(request: &mut Request) -> Result<Arc<Config>, ApiError> {
        request.get::<PersistentRead<Config>>().map_err(ApiError::from)
    }
}

impl Key for Config {
    type Value = Config;
}
