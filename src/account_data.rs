//! Account information stored for a user.

use diesel::{
    ExecuteDsl,
    ExpressionMethods,
    FilterDsl,
    LoadDsl,
    SaveChangesDsl,
    delete,
    insert,
};
use diesel::result::Error as DieselError;
use diesel::pg::PgConnection;
use iron::typemap::Key;
use ruma_identifiers::UserId;

use error::ApiError;
use schema::account_data;

/// Holds personal information/configuration for a user.
#[derive(Debug, Clone, Identifiable, Queryable)]
#[changeset_for(account_data)]
#[table_name="account_data"]
pub struct AccountData {
    /// Entry ID
    pub id: i64,
    /// The user's unique ID.
    pub user_id: UserId,
    /// The type of the data.
    pub data_type: String,
    /// The contents.
    pub content: String,
}

/// New account data, not yet saved.
#[derive(Debug)]
#[insertable_into(account_data)]
pub struct NewAccountData {
    /// The ID of the user who owns the data.
    pub user_id: UserId,
    /// The type of the data to be saved.
    pub data_type: String,
    /// The contents.
    pub content: String,
}

impl AccountData {
    /// Create new `AccountData` for a user.
    pub fn create(connection: &PgConnection, new_account_data: &NewAccountData)
    -> Result<usize, ApiError> {
        insert(new_account_data)
            .into(account_data::table)
            .execute(connection)
            .map_err(ApiError::from)
    }

    /// Look up an `AccountData` entry using the `UserId` and the data type of the data.
    pub fn find_by_uid_and_type(connection: &PgConnection, user_id: &UserId, data_type: &str)
    -> Result<AccountData, DieselError> {
        account_data::table
            .filter(account_data::user_id.eq(user_id))
            .filter(account_data::data_type.eq(data_type))
            .first(connection)
            .map(AccountData::from)
    }

    /// Update an `AccountData` entry with new content.
    pub fn update(&mut self, connection: &PgConnection, content: String)
    -> Result<(), ApiError> {
        self.content = content;

        match self.save_changes::<AccountData>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Delete all account data of a user given a `UserId`.
    pub fn delete_by_uid(connection: &PgConnection, uid: UserId)
    -> Result<usize, ApiError> {
        let rows = account_data::table
            .filter(account_data::user_id.eq(uid));

        delete(rows)
            .execute(connection)
            .map_err(ApiError::from)
    }

    /// Get all account data given a `UserId`.
    pub fn get_by_uid(connection: &PgConnection, uid: &UserId)
    -> Result<Vec<AccountData>, ApiError> {
        account_data::table
            .filter(account_data::user_id.eq(uid))
            .load::<AccountData>(connection)
            .map_err(ApiError::from)
    }
}

impl Key for AccountData {
    type Value = AccountData;
}
