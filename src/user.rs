//! Matrix users.

use diesel::{
    Connection,
    ExpressionMethods,
    FilterDsl,
    LoadDsl,
    insert,
};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use iron::typemap::Key;
use rand::{Rng, thread_rng};

use access_token::AccessToken;
use crypto::verify_password;
use error::APIError;
use schema::users;

/// A Matrix user.
#[derive(Debug, Clone, Identifiable, Queryable)]
#[changeset_for(users)]
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
    /// The user's hashed password.
    pub password_hash: String,
}

impl User {
    /// Creates a new user in the database.
    pub fn create(
        connection: &PgConnection,
        new_user: &NewUser,
        macaroon_secret_key: &Vec<u8>,
    ) -> Result<(User, AccessToken), APIError> {
        connection.transaction::<(User, AccessToken), APIError, _>(|| {
            let user: User = insert(new_user)
                .into(users::table)
                .get_result(connection)
                .map_err(APIError::from)?;

            let access_token = AccessToken::create(connection, &user.id[..], macaroon_secret_key)?;

            Ok((user, access_token))
        }).map_err(APIError::from)
    }

    /// Look up a user using the given `AccessToken`.
    pub fn find_by_access_token(connection: &PgConnection, token: &AccessToken)
    -> Result<User, APIError> {
        users::table
            .filter(users::id.eq(&token.user_id))
            .first(connection)
            .map(User::from)
            .map_err(APIError::from)
    }

    /// Look up a user using the given user ID and plaintext password.
    pub fn find_by_uid_and_password(
        connection: &PgConnection,
        id: &str,
        plaintext_password: &str,
    ) -> Result<User, APIError> {
        match users::table.filter(users::id.eq(id)).first(connection).map(User::from) {
            Ok(user) => {
                if verify_password(user.password_hash.as_bytes(), plaintext_password)? {
                    Ok(user)
                } else {
                    Err(APIError::unauthorized())
                }
            }
            Err(error) => Err(APIError::from(error)),
        }
    }

    /// Generate a random user ID.
    pub fn generate_uid() -> String {
        thread_rng().gen_ascii_chars().take(12).collect()
    }
}

impl Key for User {
    type Value = User;
}
