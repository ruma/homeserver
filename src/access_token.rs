//! User access tokens.

use base64::u8en;
use chrono::{Duration, UTC};
use diesel::{ExpressionMethods, FilterDsl, LoadDsl, Queryable, SaveChangesDsl, Table, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use iron::typemap::Key;
use macaroons::caveat::{Caveat, Predicate};
use macaroons::token::Token;

use error::APIError;
use schema::access_tokens;

/// A User access token.
#[derive(Debug, Queryable)]
#[changeset_for(access_tokens)]
pub struct AccessToken {
    /// The access token's ID.
    pub id: i64,
    /// The ID of the user who owns the access token.
    pub user_id: String,
    /// The value of the access token. This is a Base64-encoded macaroon.
    pub value: String,
    /// Whether or not the access token has been revoked.
    pub revoked: bool,
    /// The time the access token was created.
    pub created_at: PgTimestamp,
    /// The time the access token was last modified.
    pub updated_at: PgTimestamp,
}

/// A new access token, not yet saved.
#[derive(Debug)]
#[insertable_into(access_tokens)]
pub struct NewAccessToken {
    /// The ID of the user who owns the access token.
    pub user_id: String,
    /// The value of the access token. This is a Base64-encoded macaroon.
    pub value: String,
}

impl AccessToken {
    pub fn find_valid_by_token(connection: &PgConnection, token: &str)
    -> Result<AccessToken, APIError> {
        access_tokens::table
            .filter(access_tokens::value.eq(token))
            .filter(access_tokens::revoked.eq(false))
            .first(connection)
            .map(AccessToken::from)
            .map_err(APIError::from)
    }

    pub fn revoke(&mut self, connection: &PgConnection) -> Result<(), APIError> {
        self.revoked = true;

        match self.save_changes::<AccessToken>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(APIError::from(error)),
        }
    }
}

impl Key for AccessToken {
    type Value = AccessToken;
}

/// Create a new access token for the given user.
pub fn create_access_token(
    connection: &PgConnection,
    user_id: &str,
    macaroon_secret_key: &Vec<u8>,
) -> Result<AccessToken, APIError> {
    let new_access_token = NewAccessToken {
        user_id: user_id.to_string(),
        value: create_macaroon(macaroon_secret_key, user_id)?,
    };

    insert(&new_access_token)
        .into(access_tokens::table)
        .get_result(connection)
        .map_err(APIError::from)
}

fn create_macaroon(macaroon_secret_key: &Vec<u8>, user_id: &str) -> Result<String, APIError> {
    let expiration = match UTC::now().checked_add(Duration::hours(1)) {
        Some(datetime) => datetime,
        None => return Err(APIError::unknown_from_string(
            "Failed to generate access token expiration datetime.".to_string()
        )),
    };

    let token = Token::new(macaroon_secret_key, "key".as_bytes().to_owned(), vec![])
        .add_caveat(&Caveat::first_party(Predicate(
            format!("user_id = {}", user_id).as_bytes().to_owned()
        )))
        .add_caveat(&Caveat::first_party(Predicate("type = access".as_bytes().to_owned())))
        .add_caveat(&Caveat::first_party(Predicate(
            format!("time < {}", expiration).as_bytes().to_owned()
        )));

    let serialized = token.serialize();
    let encoded = u8en(&serialized)?;

    String::from_utf8(encoded).map_err(APIError::from)
}
