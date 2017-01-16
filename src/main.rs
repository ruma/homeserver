//! Ruma is a Matrix homeserver client API.

#![feature(custom_attribute, proc_macro, try_from)]
#![deny(missing_docs)]

extern crate argon2rs;
extern crate base64;
extern crate bodyparser;
extern crate chrono;
extern crate clap;
extern crate env_logger;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate iron;
#[cfg(test)] extern crate iron_test;
#[macro_use] extern crate log;
extern crate macaroons;
extern crate mount;
extern crate plugin;
extern crate persistent;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate router;
extern crate ruma_events;
extern crate ruma_identifiers;
extern crate rustc_serialize;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate time;
extern crate toml;
extern crate unicase;
extern crate url;

use clap::{App, AppSettings, SubCommand};

use config::Config;
use crypto::generate_macaroon_secret_key;
use server::Server;

#[macro_use]
pub mod middleware;
/// API endpoints as Iron handlers.
pub mod api {
    pub mod r0;
}
pub mod authentication;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
/// Models for the API's domain objects.
pub mod models;
pub mod modifier;
pub mod schema;
pub mod server;
pub mod query;
pub mod swagger;
#[cfg(test)] pub mod test;

embed_migrations!();

fn main() {
    env_logger::init().expect("Failed to initialize logger.");

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
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
            let config = match Config::from_file() {
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
