//! User access tokens.

use diesel::{LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;

use error::APIError;
use schema::access_tokens;

/// A User access token.
#[derive(Debug, Queryable)]
pub struct AccessToken {
    /// The access token's ID.
    pub id: i64,
    /// The ID of the user who owns the access token.
    pub user_id: String,
    /// The value of the access token. This is a macaroon.
    pub value: String,
    /// The time the access token was created.
    pub created_at: PgTimestamp,
}

/// A new access token, not yet saved.
#[derive(Debug)]
#[insertable_into(access_tokens)]
pub struct NewAccessToken {
    /// The ID of the user who owns the access token.
    pub user_id: String,
    /// The value of the access token. This is a macaroon.
    pub value: String,
}

/// Create a new access token for the given user.
pub fn create_access_token<'a>(
    connection: &'a PgConnection,
    user_id: &'a str,
) -> Result<AccessToken, APIError> {
    let new_access_token = NewAccessToken {
        user_id: user_id.to_owned(),
        value: "fake access token".to_owned(),
    };

    insert(&new_access_token)
        .into(access_tokens::table)
        .get_result(connection)
        .map_err(APIError::from)
}
