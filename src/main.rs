extern crate clap;
extern crate hyper;
extern crate iron;
extern crate mount;
extern crate router;

mod api;
mod server;

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
