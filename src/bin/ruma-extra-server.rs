extern crate clap;
extern crate env_logger;
extern crate ruma;

use clap::{App, AppSettings, Arg, SubCommand};

use ruma::config::Config;
use ruma::server::Server;

fn main() {
    if let Err(error) = env_logger::init() {
        eprintln!("Failed to initialize logger: {}", error);
    }

    let matches = App::new("ruma-extra-server")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Extra APIs for Ruma.")
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("run").about("Runs the server").arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("PATH")
                    .help("Path to a configuration file")
                    .takes_value(true),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(submatches)) => {
            let config = match Config::from_file(submatches.value_of("config")) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("Failed to load configuration file: {}", error);

                    return;
                }
            };

            let server = Server::new(&config).mount_extra();

            if let Err(error) = server.run() {
                eprintln!("Server failed: {}", error);
            }
        }
        _ => println!("{}", matches.usage()),
    };
}
