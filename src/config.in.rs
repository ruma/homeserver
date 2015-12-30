use std::fs::File;
use std::io::Read;

use toml::decode_str;

use error::CLIError;

#[derive(Deserialize)]
pub struct Config {
    pub postgres_url: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Config, CLIError> {
        let mut file = try!(File::open(path));
        let mut contents = String::new();
        try!(file.read_to_string(&mut contents));

        match decode_str(&contents) {
            Some(config) => Ok(config),
            None => Err(CLIError::new("failed to decode config file")),
        }
    }
}
