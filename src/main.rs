extern crate bodyparser;
extern crate clap;
extern crate diesel;
extern crate hyper;
extern crate iron;
extern crate mount;
extern crate persistent;
extern crate router;
extern crate serde;
extern crate serde_json;
extern crate toml;

mod api {
    pub mod r0 {
        pub mod authentication;
    }
}
mod config;
mod db;
mod error;
mod middleware;
mod modifier;
mod server;

use clap::{App, AppSettings, SubCommand};

use config::Config;
use server::Server;

fn main() {
    let matches = App::new("ruma")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A Matrix homeserver")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("start")
                .about("Starts the Ruma server")
        )
        .get_matches();

    match matches.subcommand() {
        ("start", Some(_matches)) => {
            let config = match Config::load("ruma.toml") {
                Ok(config) => config,
                Err(error) => {
                    println!("Failed to connect to PostgreSQL: {}", error);

                    return;
                }
            };
            let server = Server::new(&config);
            if let Err(error) = server.start() {
                println!("{}", error);
            }
        },
        _ => println!("{}", matches.usage()),
    };
}
