# Ruma

[![Build Status](https://travis-ci.org/ruma/ruma.svg?branch=master)](https://travis-ci.org/ruma/ruma)

**Ruma** is a [Matrix](https://matrix.org/) homeserver written in [Rust](https://www.rust-lang.org/).

If you're interested in the project, please take a look at the [Ruma website](https://www.ruma.io/), follow [ruma_io](https://twitter.com/ruma_io) on Twitter and chat with us in [#ruma:matrix.org](https://matrix.to/#/#ruma:matrix.org) on Matrix (also accessible via [#ruma](https://webchat.freenode.net/?channels=ruma) on the freenode IRC network.)

## Status

**Ruma is not currently being maintained and cannot realistically be used in its current state.**

The goal of Ruma as a project is to provide a complete implementation of a Matrix homeserver, a Matrix identity server, a Matrix client library, and Matrix application services.
This repository in particular aims to implement a Matrix homeserver.
The Ruma homeserver will be packaged as a single executable for small-scale deployments, and as multiple executables for large deployments that need to scale different parts of the homeserver independently.
Additional Matrix libraries used by Ruma can be found in the [Ruma organization on GitHub](https://github.com/ruma).

For a detailed view of which Matrix APIs are supported by Ruma so far, see the [STATUS](STATUS.md) document.

## Development

Ruma includes a development setup using [Docker](https://www.docker.com/).
To install Docker, see the installation instructions for [OS X](https://docs.docker.com/docker-for-mac/), [Linux](https://docs.docker.com/install/), or [Windows](https://docs.docker.com/docker-for-windows/).
(Note that both Docker and Docker Compose are needed, but the standard ways of installing include both.)

**Note**: `docker-compose` version 1.6 or higher and `docker-engine` version 1.10.0 or higher are required.

Cargo is the main entrypoint for development.
Use the `script/cargo` shell script as you would normally use plain `cargo`.
This will run the Cargo command inside a Docker container that has Rust and other dependencies already installed.
It will automatically start a PostgreSQL database inside a container as well.
The first time you run a command with `script/cargo`, it will take some time to download the Docker images.

To build Ruma, run `script/cargo build --bin ruma`.
The application will be written to `target/debug/ruma`.
You can also build and run Ruma in one step with `script/cargo run --bin ruma`.
(When run via Cargo, arguments to `ruma` itself must come after two dashes, e.g. `script/cargo run --bin ruma -- run`.)

## Minimum Rust version

Ruma requires Rust 1.34 or later.

### Developing without Docker

Docker is used to make everyone's life easier including packaging Rust along with Ruma's other dependencies, and managing test PostgreSQL databases, all without assuming anything about the host system.
If you really want to avoid Docker, it's up to you to configure your development environment to match the assumptions made by code in Ruma.
In particular, this means at least the minimum version of Rust, all the system-level dependencies such as libsodium, and a PostgreSQL installation with suitable permissions available at the address and port used in `src/test.rs`.

## Documentation

To generate API documentation for Ruma, run `script/cargo doc`.
Then open `target/doc/ruma/index.html` in your browser.
Note that this documentation is for Ruma's internal Rust code, not the public-facing Matrix API.
User-facing documentation will live on the [Ruma website](https://www.ruma.io/).

## Testing

Ruma includes an integration test suite.
Once Docker is installed, run `script/cargo test` to run the test suite.

## Configuration

Ruma requires a configuration file named `ruma.json`, `ruma.toml`, or `ruma.yaml`/`ruma.yml` written in JSON, TOML, or YAML, respectively.
This file should be in the working directory `ruma` is executed from.
Ruma will attempt to load the configuration file in that same order, stopping at the first one it finds.
A configuration file would look something like this, in the JSON format:

``` json
{
  "version": "1",
  "domain": "example.com",
  "macaroon_secret_key": "qbnabRiFu5fWzoijGmc6Kk2tRox3qJSWvL3VRl4Vhl8=",
  "postgres_url": "postgres://username:password@example.com:5432/ruma"
}
```

The complete list of attributes in the configuration is as follows:

* **bind_address** (string, default: "127.0.0.1"):
  The network address where the server should listen for connections.
* **bind_port** (string, default: "3000"):
  The network port where the server should listen for connections.
* **domain** (string, required):
  The DNS name where clients can reach the server.
  Used as the hostname portion of user IDs.
* **macaroon_secret_key** (string, required):
  The secret key used for generating [Macaroons](https://research.google.com/pubs/pub41892.html).
  Must be 32 cryptographically random bytes, encoded as a Base64 string.
  Changing this value will invalidate any previously generated macaroons, effectively ending all user sessions.
* **postgres_url** (string, required):
  A [PostgreSQL connection string](http://www.postgresql.org/docs/current/static/libpq-connect.html#LIBPQ-CONNSTRING) for Ruma's PostgreSQL database.
* **version** (string, required):
  The version of the Ruma configuration file format that this configuration represents.
  This field allows Ruma to make backwards-incompatible changes to the configuration file format over time without breaking existing deployments.
  Currently the only valid value is "1".

## Usage

```
ruma 0.1.0
A Matrix homeserver.

USAGE:
    ruma [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help      Prints this message or the help message of the given subcommand(s)
    run       Runs the Ruma server
    secret    Generates a random value to be used as a macaroon secret key
```

Before you run `ruma run`, make sure you have a configuration file in the working directory named `ruma.json` and that a PostgreSQL server is running and available at the location specified in the configuration file.
Ruma will automatically create the database (if it doesn't already exist) and manage the database schema.
You are responsible for providing Ruma with a valid PostgreSQL server URL and role that can perform these operations.

## Swagger

Ruma includes an HTTP endpoint to serve [Swagger](http://swagger.io/) data at http://example.com/ruma/swagger.json (substituting the host and port of your Ruma server for example.com, of course.)
Point a copy of [Swagger UI](https://github.com/swagger-api/swagger-ui) at this URL to see complete documentation for the Matrix client API.
Note that Ruma does not actually implement all these API endpoints yet.

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) document.

## Dedication

Ruma is dedicated to my best friend, Tamara Boyens, who passed away in January 2017.
She and I talked online for hours every day.
She was a large part of my motivation in starting Ruma, because our online communication was where we spent the most time together after we both moved away from the city where we met, and we were always looking for a system that would fix our grievances with all the subpar choices we had for chatting.

— Jimmy Cuadra

## License

[MIT](http://opensource.org/licenses/MIT)
