use std::fs::File;
use std::io::Read;

use serde_json::from_str;

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

        from_str(&contents).map_err(|error| From::from(error))
    }
}
