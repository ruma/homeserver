# Ruma

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

Ruma currently requires the nightly version of Rust, primarily because it makes heavy use of the code generation features of [Diesel](https://github.com/sgrif/diesel) and [Serde](https://github.com/serde-rs/serde), which use compiler plugins, an unstable Rust feature.
This particular use of compiler plugins is likely to be replaced by a new macro system currently being developed by Nick Cameron (see [libmacro](http://www.ncameron.org/blog/libmacro/) and the [procedural macros RFC](https://github.com/rust-lang/rfcs/pull/1566)), but is probably a very long way off from making it to stable Rust.

To install a nightly version of Rust, use [rustup](https://www.rustup.rs/) or head over to the [Rust Downloads](https://www.rust-lang.org/downloads.html) page.

To build Ruma, run `cargo build`. The application will be written to `target/debug/ruma`.
You can also build and run Ruma in one step with `cargo run`.
(When run via Cargo, arguments to `ruma` itself must come after two dashes, e.g. `cargo run -- run`.)

To generate API documentation, run `cargo doc`.
Then open `target/doc/ruma/index.html` in your browser.
Note that this documentation is for Ruma's internal Rust code, not the public-facing Matrix API.

## Testing

Ruma includes an integration test suite.
The test suite relies on Docker for ephemeral PostgreSQL databases.
To install Docker, see the installation instructions for [OS X](https://docs.docker.com/mac/), [Linux](https://docs.docker.com/linux/), or [Windows](https://docs.docker.com/windows/).
Once Docker is installed, run `make` to run the test suite.

## Configuration

Ruma requires a configuration file named `ruma.json` in the directory `ruma` is executed from.
The file should contain a JSON object that looks something like this:

``` json
{
  "domain": "example.com",
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
