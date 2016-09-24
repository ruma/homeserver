# Ruma

[![Build Status](https://travis-ci.org/ruma/ruma.svg?branch=master)](https://travis-ci.org/ruma/ruma)

**Ruma** is a [Matrix](https://matrix.org/) homeserver written in [Rust](https://www.rust-lang.org/).

If you're interested in the project, please follow [ruma_io](https://twitter.com/ruma_io) on Twitter and join us in [#ruma:matrix.org](https://vector.im/beta/#/room/#ruma:matrix.org) on Matrix (also accessible via [#ruma](https://webchat.freenode.net/?channels=ruma) on the freenode IRC network.)

## Status

The goal of Ruma as a project is to provide a complete implementation of a Matrix homeserver, a Matrix identity server, a Matrix client library, and Matrix application services.
This repository in particular aims to implement the client API of a Matrix homeserver.
The homeserver federation API lives at [ruma/ruma-federation](https://github.com/ruma/ruma-federation), but will not be actively developed until the federation API specification has stabilized and the client API is in a practically useful state.
This separation of the two homeserver APIs allows users to run a private homeserver without federation if they choose, and to scale the infrastructure for their client and federation APIs separately if they choose to participate in a larger Matrix network.
Additional Matrix libraries used by Ruma can be found in the [Ruma organization on GitHub](https://github.com/ruma).

Ruma is currently pre-alpha and cannot realistically be used from a standard Matrix client, but it's getting closer every week!

For a detailed view of which Matrix APIs are supported by Ruma so far, see the [STATUS](STATUS.md) document.

## Development

Ruma includes a development setup using [Docker](https://www.docker.com/).
To install Docker, see the installation instructions for [OS X](https://docs.docker.com/mac/), [Linux](https://docs.docker.com/linux/), or [Windows](https://docs.docker.com/windows/).
(Note that both Docker and Docker Compose are needed, but the standard ways of installing include both.)

Cargo is the main entrypoint for development.
Use the `script/cargo` shell script as you would normally use plain `cargo`.
This will run the Cargo command inside a Docker container that has  Rust and other dependencies already installed.
It will automatically start a PostgreSQL database inside a container as well.
The first time you run a command with `script/cargo`, it will take some time to download the Docker images.

To build Ruma, run `script/cargo build`.
The application will be written to `target/debug/ruma`.
You can also build and run Ruma in one step with `script/cargo run`.
(When run via Cargo, arguments to `ruma` itself must come after two dashes, e.g. `script/cargo run -- run`.)

### Nightly Rust

Ruma currently requires the nightly version of Rust because it uses the following unstable features, listed below with links to the GitHub issues tracking stabilization:

* `custom_attribute`, `custom_derive`, `plugin`: These will all be replaced and stabilized soon by [Macros 1.1](https://github.com/rust-lang/rust/issues/35900).
* [`question_mark`](https://github.com/rust-lang/rust/issues/31436)
* [`specialization`](https://github.com/rust-lang/rust/issues/31844)
* [`try_from`](https://github.com/rust-lang/rust/issues/33417)

When all of these features are stabilized, Ruma will target stable Rust.

### Developing without Docker

Docker is used to make everyone's life easier by pinning a compatible version of nightly Rust and managing test PostgreSQL databases without assuming anything about the host system.
If you really want to avoid Docker, it's up to you to configure your development environment to match the assumptions made by code in Ruma.
In particular, this means a version of the nightly Rust compiler that can compile Ruma given the current Cargo.lock and a PostgreSQL installation with suitable permissions available at the address and port used in `src/test.rs`.
You can find the version of nightly Rust used in the Docker setup by looking at the Dockerfile for Ruma's [development Docker image](https://github.com/ruma/docker-ruma-dev).
Look at the line that installs rustup for the date.
It will look something like this:

``` bash
./rustup-init -y --no-modify-path --default-toolchain nightly-YYYY-MM-DD
```

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

## Usage

```
ruma 0.1.0
A Matrix homeserver client API

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

If you don't need this functionality, you can create a smaller `ruma` binary by building Ruma by running:

``` bash
cargo build --no-default-features
```

The Swagger endpoint is compiled conditionally when the "swagger" Cargo feature is enabled.

## Contributing

See the [CONTRIBUTING](CONTRIBUTING.md) document.

## License

[MIT](http://opensource.org/licenses/MIT)
