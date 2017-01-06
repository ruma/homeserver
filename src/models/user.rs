//! Matrix users.

use diesel::{
    Connection,
    FindDsl,
    LoadDsl,
    SaveChangesDsl,
    insert,
};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::Error as DieselError;
use iron::typemap::Key;
use ruma_identifiers::UserId;

use crypto::verify_password;
use error::ApiError;
use models::access_token::AccessToken;
use schema::users;

/// A Matrix user.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "users"]
pub struct User {
    /// The user's unique ID.
    pub id: UserId,
    /// An [Argon2](https://en.wikipedia.org/wiki/Argon2) hash of the user's password.
    pub password_hash: String,
    /// Whether or not the user has the ability to login.
    pub active: bool,
    /// The time the user was created.
    pub created_at: PgTimestamp,
    /// The time the user was last modified.
    pub updated_at: PgTimestamp,
}

/// A new Matrix user, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    /// The user's unique ID.
    pub id: UserId,
    /// The user's hashed password.
    pub password_hash: String,
}

impl User {
    /// Creates a new user in the database.
    pub fn create(
        connection: &PgConnection,
        new_user: &NewUser,
        macaroon_secret_key: &[u8],
    ) -> Result<(User, AccessToken), ApiError> {
        connection.transaction::<(User, AccessToken), ApiError, _>(|| {
            let user: User = insert(new_user)
                .into(users::table)
                .get_result(connection)
                .map_err(ApiError::from)?;

            let access_token = AccessToken::create(connection, &user.id, macaroon_secret_key)?;

            Ok((user, access_token))
        }).map_err(ApiError::from)
    }

    /// Verify that a `User` with the given `UserId` and plaintext password exists.
    pub fn verify(
        connection: &PgConnection,
        id: &UserId,
        plaintext_password: &str,
    ) -> Result<User, ApiError> {
        match User::find_active_user(connection, id)? {
            Some(user) => {
                if !verify_password(user.password_hash.as_bytes(), plaintext_password)? {
                    return Err(ApiError::unauthorized("Invalid credentials".to_string()))
                }

                Ok(user)
            },
            None => {
                Err(ApiError::not_found(format!("The user {} was not found on this server", id)))
            }
        }
    }

    /// Look up a registered `User` using the given `UserId`.
    pub fn find_registered_user(connection: &PgConnection, id: &UserId)
    -> Result<Option<User>, ApiError> {
        let result = users::table
            .find(id)
            .get_result(connection);

        match result {
            Ok(user) => Ok(Some(user)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Look up an active `User` using the given `UserId`.
    ///
    /// A user stops being active when he deactivates his account.
    pub fn find_active_user(connection: &PgConnection, id: &UserId)
    -> Result<Option<User>, ApiError> {
        match User::find_registered_user(connection, id)? {
            Some(ref user) if user.active => Ok(Some(user.clone())),
            _ => Ok(None)
        }
    }

    /// Remove the user's ability to login.
    pub fn deactivate(&mut self, connection: &PgConnection) -> Result<(), ApiError> {
        self.active = false;

        match self.save_changes::<User>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(ApiError::from(error)),
        }
    }
}

impl Key for User {
    type Value = User;
}
