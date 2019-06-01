//! Ruma is a Matrix homeserver.

#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    warnings
)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[cfg(test)]
extern crate env_logger;
#[cfg(test)]
extern crate iron_test;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

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
pub mod query;
pub mod schema;
pub mod server;
pub mod swagger;
#[cfg(test)]
pub mod test;

embed_migrations!();
