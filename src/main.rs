extern crate clap;
extern crate env_logger;
extern crate ruma;

use clap::{App, AppSettings, SubCommand};

use ruma::config::load;
use ruma::crypto::generate_macaroon_secret_key;
use ruma::server::Server;

fn main() {
    env_logger::init().expect("Failed to initialize logger.");

    let matches = App::new("ruma")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A Matrix client-server API")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs the Ruma server")
        )
        .subcommand(
            SubCommand::with_name("secret")
                .about("Generates a random value to be used as a macaroon secret key")
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(_)) => {
            let config = match load("ruma.json") {
                Ok(config) => config,
                Err(error) => {
                    println!("Failed to load configuration file: {}", error);

                    return;
                }
            };

            match Server::new(&config) {
                Ok(server) => {
                    if let Err(error) = server.run() {
                        println!("{}", error);
                    }
                },
                Err(error) => {
                    println!("Failed to create server: {}", error);

                    return;
                }
            }
        }
        ("secret", Some(_)) => match generate_macaroon_secret_key() {
            Ok(key) => println!("{}", key),
            Err(error) => println!("Failed to generate macaroon secret key: {}", error),
        },
        _ => println!("{}", matches.usage()),
    };
}
