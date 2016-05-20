//! Ruma is a server for Matrix.org's client-server API.

#![feature(custom_attribute, custom_derive, plugin)]
#![plugin(diesel_codegen)]
#![plugin(serde_macros)]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate argon2rs;
extern crate base64;
extern crate bodyparser;
extern crate chrono;
extern crate clap;
extern crate env_logger;
#[macro_use] extern crate diesel;
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
extern crate serde;
extern crate serde_json;
extern crate unicase;

use clap::{App, AppSettings, SubCommand};

use config::load;
use crypto::generate_macaroon_secret_key;
use server::Server;

pub mod access_token;
/// API endpoints as Iron handlers.
pub mod api {
    /// API endpoints for the 0.x.x version of the Matrix spec.
    pub mod r0 {
        pub mod login;
        pub mod registration;
        pub mod versions;
    }
}
pub mod authentication;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod middleware;
pub mod modifier;
pub mod schema;
pub mod server;
pub mod swagger;
#[cfg(test)] pub mod test;
pub mod user;

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
