//! Matrix users.

use diesel::pg::data_types::PgTimestamp;

use schema::users;

/// A Matrix user.
#[derive(Debug, Queryable)]
pub struct User {
    /// The user's username (localpart).
    pub id: String,
    /// An [Argon2](https://en.wikipedia.org/wiki/Argon2) hash of the user's password.
    pub password_hash: String,
    /// The time the user was created.
    pub created_at: PgTimestamp,
    /// The time the user was last modified.
    pub updated_at: PgTimestamp,
}

/// A new Matrix user, not yet saved.
#[derive(Debug)]
#[insertable_into(users)]
pub struct NewUser {
    /// The user's username (localpart).
    pub id: String,
    /// The user's password as plaintext.
    pub password_hash: String,
}
