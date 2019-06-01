//! Account information stored for a user.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::pg::PgConnection;
use iron::typemap::Key;
use ruma_identifiers::{UserId, RoomId};

use error::ApiError;
use schema::{account_data, room_account_data};

/// Holds personal information/configuration for a user.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "account_data"]
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
#[derive(Debug, Insertable)]
#[table_name = "account_data"]
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
    -> Result<AccountData, ApiError> {
        diesel::insert_into(account_data::table)
            .values(new_account_data)
            .get_result(connection)
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
    -> Result<AccountData, ApiError> {
        self.content = content;

        self.save_changes::<AccountData>(connection)
            .map_err(ApiError::from)
    }

    /// Delete all account data of a user given a `UserId`.
    pub fn delete_by_uid(connection: &PgConnection, uid: &UserId)
    -> Result<usize, ApiError> {
        let rows = account_data::table
            .filter(account_data::user_id.eq(uid));

        diesel::delete(rows)
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

    /// Update an existing entry or create a new one.
    pub fn upsert(connection: &PgConnection, new_data: &NewAccountData)
    -> Result<AccountData, ApiError> {
        match AccountData::find_by_uid_and_type(
            connection,
            &new_data.user_id,
            &new_data.data_type
        ) {
            Ok(mut saved) => saved.update(connection, new_data.content.clone()),
            Err(err) => {
                match err {
                    DieselError::NotFound => AccountData::create(connection, new_data),
                    _ => Err(ApiError::from(err))
                }
            }
        }
    }
}

impl Key for AccountData {
    type Value = AccountData;
}

/// Holds user's information/configuration per room.
#[derive(AsChangeset, Clone, Debug, Identifiable, Queryable)]
#[table_name = "room_account_data"]
pub struct RoomAccountData {
    /// Entry ID
    pub id: i64,
    /// The user's unique ID.
    pub user_id: UserId,
    /// The room's unique ID.
    pub room_id: RoomId,
    /// The type of the data.
    pub data_type: String,
    /// The contents.
    pub content: String,
}

/// New room account data, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "room_account_data"]
pub struct NewRoomAccountData {
    /// The ID of the user who owns the data.
    pub user_id: UserId,
    /// The room's unique ID.
    pub room_id: RoomId,
    /// The type of the data to be saved.
    pub data_type: String,
    /// The contents.
    pub content: String,
}

impl RoomAccountData {
    /// Create new `RoomAccountData` for a user.
    pub fn create(connection: &PgConnection, new_account_data: &NewRoomAccountData)
    -> Result<RoomAccountData, ApiError> {
        diesel::insert_into(room_account_data::table)
            .values(new_account_data)
            .get_result(connection)
            .map_err(ApiError::from)
    }

    /// Look up a `RoomAccountData` entry.
    pub fn find(connection: &PgConnection, uid: &UserId, rid: &RoomId, data_type: &str)
    -> Result<RoomAccountData, DieselError> {
        room_account_data::table
            .filter(room_account_data::user_id.eq(uid))
            .filter(room_account_data::room_id.eq(rid))
            .filter(room_account_data::data_type.eq(data_type))
            .first(connection)
            .map(RoomAccountData::from)
    }

    /// Update an `RoomAccountData` entry with new content.
    pub fn update(&mut self, connection: &PgConnection, content: String)
    -> Result<RoomAccountData, ApiError> {
        self.content = content;

        self.save_changes::<RoomAccountData>(connection)
            .map_err(ApiError::from)
    }

    /// Delete all account data for a user given a `UserId`.
    pub fn delete_by_uid(connection: &PgConnection, uid: &UserId)
    -> Result<usize, ApiError> {
        let rows = room_account_data::table
            .filter(room_account_data::user_id.eq(uid));

        diesel::delete(rows)
            .execute(connection)
            .map_err(ApiError::from)
    }

    /// Update an existing entry or create a new one.
    pub fn upsert(connection: &PgConnection, new_data: &NewRoomAccountData)
    -> Result<RoomAccountData, ApiError> {
        match RoomAccountData::find(
            connection,
            &new_data.user_id,
            &new_data.room_id,
            &new_data.data_type
        ) {
            Ok(mut saved) => saved.update(connection, new_data.content.clone()),
            Err(err) => {
                match err {
                    DieselError::NotFound => RoomAccountData::create(connection, new_data),
                    _ => Err(ApiError::from(err))
                }
            }
        }
    }
}

impl Key for RoomAccountData {
    type Value = RoomAccountData;
}
