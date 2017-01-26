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

/// Default paths where Ruma will look for a configuration file if left unspecified.
static DEFAULT_CONFIG_FILES: [&'static str; 4] = ["ruma.json", "ruma.toml", "ruma.yaml", "ruma.yml"];

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
    ///
    /// If a path is given, it will try to load the configuration there.
    /// Otherwise, try to load a file from the defaults locations.
    pub fn from_file(path: Option<&str>) -> Result<Config, CliError> {
        let config_path = if let Some(ref path_str) = path {
            let path = Path::new(path_str);
            if !path.is_file() {
                return Err(CliError::new(format!("Configuration file `{}` not found.", path_str)));
            }
            path
        } else {
            DEFAULT_CONFIG_FILES.iter()
                .map(Path::new)
                .find(|path| path.is_file())
                .ok_or(CliError::new("No configuration file was found."))?
        };

        let config = match config_path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => Self::load_json(&config_path),
            Some("toml") => Self::load_toml(&config_path),
            Some("yml") | Some("yaml") => Self::load_yaml(&config_path),
            _ => Err(CliError::new("Unsupported configuration file format")),
        }?;

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
    fn load_json(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
        match serde_json::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Load the `RawConfig` from a TOML configuration file.
    fn load_toml(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
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
    fn load_yaml(path: &Path) -> Result<RawConfig, CliError> {
        let contents = Self::read_file_contents(path);
        match serde_yaml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(CliError::from(error)),
        }
    }

    /// Read the contents of a file.
    fn read_file_contents(path: &Path) -> String {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    /// Extract the `Config` stored in the request.
    pub fn from_request(request: &mut Request) -> Result<Arc<Config>, ApiError> {
        request.get::<PersistentRead<Config>>().map_err(ApiError::from)
    }
}

impl Key for Config {
    type Value = Config;
}
