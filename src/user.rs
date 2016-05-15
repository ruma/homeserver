//! Matrix users.

use diesel::{Connection, ExpressionMethods, FilterDsl, LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use rand::{Rng, thread_rng};

use access_token::{AccessToken, create_access_token};
use error::APIError;
use schema::users;

/// A Matrix user.
#[derive(Debug, Clone, Queryable)]
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

pub fn load_user(connection: &PgConnection, id: &str, password_hash: &str)
-> Result<User, APIError> {
    users::table
        .filter(users::id.eq(id))
        .filter(users::password_hash.eq(password_hash))
        .first(connection)
        .map_err(|error| APIError::from(error))
}

/// Insert a new user in the database.
pub fn insert_user(
    connection: &PgConnection,
    new_user: &NewUser,
    macaroon_secret_key: &Vec<u8>,
) -> Result<(User, AccessToken), APIError> {
    connection.transaction::<(User, AccessToken), APIError, _>(|| {
        let user: User = try!(
            insert(new_user).into(users::table).get_result(connection).map_err(APIError::from)
        );

        let access_token = try!(create_access_token(connection, &user.id[..], macaroon_secret_key));

        Ok((user, access_token))
    }).map_err(APIError::from)
}

/// Generate a random user ID.
pub fn generate_user_id() -> String {
    thread_rng().gen_ascii_chars().take(12).collect()
}
