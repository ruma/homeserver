//! Matrix transaction.

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;

use crate::error::ApiError;
use crate::schema::transactions;

/// A Transaction.
#[derive(AsChangeset, Clone, Debug, Identifiable, Insertable, Queryable)]
#[primary_key(path, access_token)]
#[table_name = "transactions"]
pub struct Transaction {
    /// The full path of the endpoint used for the transaction.
    pub path: String,
    /// The access token used.
    pub access_token: String,
    /// The serialized response of the endpoint. It should be used
    /// as the response on future requests.
    pub response: String,
}

impl Transaction {
    /// Create a new transaction entry.
    pub fn create(
        connection: &PgConnection,
        path: String,
        access_token: String,
        response: String,
    ) -> Result<Self, ApiError> {
        let new_transaction = Self {
            path,
            access_token,
            response,
        };

        diesel::insert_into(transactions::table)
            .values(&new_transaction)
            .get_result(connection)
            .map_err(ApiError::from)
    }

    /// Look up a transaction with the url path of the endpoint and the access token.
    pub fn find(
        connection: &PgConnection,
        path: &str,
        access_token: &str,
    ) -> Result<Option<Self>, ApiError> {
        let transaction = transactions::table
            .find((path, access_token))
            .get_result(connection);

        match transaction {
            Ok(transaction) => Ok(Some(transaction)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }
}
