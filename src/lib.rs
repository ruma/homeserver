//! Ruma is a Matrix homeserver.

#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    warnings
)]
#![warn(
    clippy::empty_line_after_outer_attr,
    clippy::expl_impl_clone_on_copy,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::match_same_arms,
    clippy::mem_forget,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::mut_mut,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::single_match_else,
    clippy::unicode_not_nfc,
    clippy::use_self,
    clippy::used_underscore_binding,
    clippy::wrong_pub_self_convention,
    clippy::wrong_self_convention
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
