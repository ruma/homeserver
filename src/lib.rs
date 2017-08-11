//! Ruma is a Matrix homeserver.

#![feature(try_from)]
#![deny(missing_docs)]

extern crate argon2rs;
extern crate base64;
extern crate bodyparser;
extern crate chrono;
extern crate clap;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[cfg(test)] extern crate env_logger;
extern crate iron;
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
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate toml;
extern crate unicase;
extern crate url;

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
