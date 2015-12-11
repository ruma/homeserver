extern crate bodyparser;
extern crate clap;
extern crate hyper;
extern crate iron;
extern crate mount;
extern crate persistent;
extern crate router;
extern crate rustc_serialize;
extern crate serde;
extern crate serde_json;

mod api {
    pub mod r0 {
        pub mod authentication;
    }
}
mod error;
mod middleware;
mod server;
mod repository;

use clap::{App, AppSettings, SubCommand};

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
            let server = Server::new();
            if let Err(error) = server.start() {
                println!("{}", error);
            }
        },
        _ => println!("{}", matches.usage()),
    };
}
