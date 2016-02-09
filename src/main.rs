//! Ruma is a server for Matrix.org's client-server API.

#![feature(custom_attribute, custom_derive, plugin)]
#![plugin(diesel_codegen)]
#![plugin(serde_macros)]

extern crate argon2rs;
extern crate base64;
extern crate bodyparser;
extern crate clap;
extern crate crypto;
#[macro_use] extern crate diesel;
extern crate env_logger;
extern crate hyper;
#[macro_use] extern crate iron;
#[macro_use] extern crate log;
extern crate mount;
extern crate persistent;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate router;
extern crate serde;
extern crate serde_json;

/// API endpoints as Iron handlers.
pub mod api {
    /// API endpoints for the 0.x.x version of the Matrix spec.
    pub mod r0 {
        pub mod authentication;
        pub mod versions;
    }
}
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod modifier;
pub mod schema;
pub mod server;
pub mod user;

use clap::{App, AppSettings, SubCommand};

use config::load;
use server::Server;

fn main() {
    env_logger::init().expect("Failed to initialize logger.");

    let matches = App::new("ruma")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A server for Matrix.org's client-server API.")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("start")
                .about("Starts the Ruma server")
        )
        .get_matches();

    match matches.subcommand() {
        ("start", Some(_matches)) => {
            let config = match load("ruma.json") {
                Ok(config) => config,
                Err(error) => {
                    println!("Failed to load configuration file: {}", error);

                    return;
                }
            };

            match Server::new(&config) {
                Ok(server) => {
                    if let Err(error) = server.start() {
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
