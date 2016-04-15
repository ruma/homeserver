extern crate clap;
extern crate env_logger;
extern crate ruma;

use clap::{App, AppSettings, SubCommand};

use ruma::config::load;
use ruma::server::Server;

fn main() {
    env_logger::init().expect("Failed to initialize logger.");

    let matches = App::new("ruma")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A server for Matrix.org's client-server API.")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs the Ruma server")
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(_matches)) => {
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
        },
        _ => println!("{}", matches.usage()),
    };
}
