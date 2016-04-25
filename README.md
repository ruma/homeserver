# Ruma

**Ruma** is a server for [Matrix](https://matrix.org/)'s client-server API written in [Rust](https://www.rust-lang.org/).

## Status

The goal of Ruma as a project is to provide a complete implementation of a Matrix homeserver, a Matrix client library, and possibly various application services according to the Matrix specifications.
This repository in particular aims to implement the client-server portion of the Matrix homeserver.
The homeserver federation API lives at [ruma/ruma-federation](https://github.com/ruma/ruma-federation), but will not be actively developed until the federation API specification has stabilized and the client-server API is in a practically useful state.
Additional components can be found in the [Ruma organization on GitHub](https://github.com/ruma).

This project is currently very new, experimental and is likely to change drastically.
In addition to Ruma itself being new, Rust is a very young language, and its library ecosystem is very young as well.
Currently, a large portion of development time for Ruma is spent contributing missing or incomplete functionality to other libraries in the Rust ecosystem that are needed by Ruma.

Initial efforts on the Ruma codebase itself will be focused on the user registration and login system.

## Development

Ruma currently requires the nightly version of Rust, primarily because it makes heavy use of the code generation features of [Diesel](https://github.com/sgrif/diesel) and [Serde](https://github.com/serde-rs/serde), which use compiler plugins, an unstable Rust feature.
This particular use of compiler plugins is likely to be replaced by a new macro system currently being developed by Nick Cameron (see [libmacro](http://www.ncameron.org/blog/libmacro/)), but is probably a very long way off from making it to stable Rust.

To install a nightly version of Rust, head over to the [Rust Downloads](https://www.rust-lang.org/downloads.html) page.

Ruma also requires the C library [libsodium](https://github.com/jedisct1/libsodium) for some cryptographic operations.
Install libsodium via your method of choice before building Ruma.
Packages for libsodium are available through most system package managers.

To build Ruma, run `cargo build`. The application will be written to `target/debug/ruma`.
You can also build and run Ruma in one step with `cargo run`.

To generate API documentation, run `cargo doc`. Then open `target/doc/ruma/index.html` in your browser.
Note that this documentation is for Ruma's internal Rust code, not the public-facing Matrix API.

## Testing

Ruma includes an integration test suite.
The test suite relies on Docker for ephemeral PostgreSQL databases.
To install Docker, see the installation instructions for [OS X](https://docs.docker.com/mac/), [Linux](https://docs.docker.com/linux/), or [Windows](https://docs.docker.com/windows/).
Once Docker is installed, run `cargo test` to run the test suite.

## Configuration

Ruma requires a configuration file named `ruma.json` in the directory `ruma` is executed from.
The file should contain a JSON object that looks something like this:

``` json
{
  "domain": "example.com",
  "postgres_url": "postgres://jimmy@localhost:5432/postgres"
}
```

The complete schema for the configuration file is documented through the Rust API docs for `ruma::config::Config`.

## Usage

```
ruma 0.1.0
A Matrix client-server API

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

Before you run `ruma run`, make sure you have a configuration file in the working directory named `ruma.json` and that a PostgreSQL server is running and available at the location specified in the configuration file. Ruma will automatically create the database (if it doesn't already exist) and manage the database schema. You are responsible for providing Ruma with a valid PostgreSQL server URL and role that can perform these operations.

## License

[MIT](http://opensource.org/licenses/MIT)
