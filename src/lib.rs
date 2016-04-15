//! Ruma is a server for Matrix.org's client-server API.

#![feature(custom_attribute, custom_derive, plugin)]
#![plugin(diesel_codegen)]
#![plugin(serde_macros)]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate argon2rs;
extern crate base64;
extern crate bodyparser;
#[macro_use] extern crate diesel;
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

pub mod access_token;
/// API endpoints as Iron handlers.
pub mod api {
    /// API endpoints for the 0.x.x version of the Matrix spec.
    pub mod r0 {
        pub mod registration;
        pub mod versions;
    }
}
pub mod authentication;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod middleware;
pub mod modifier;
pub mod schema;
pub mod server;
pub mod user;
