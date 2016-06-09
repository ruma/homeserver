//! Human-readable aliases for room IDs.

use diesel::{LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;

use error::APIError;
use schema::room_aliases;

/// A new room alias, not yet saved.
#[derive(Debug)]
#[insertable_into(room_aliases)]
pub struct NewRoomAlias {
    /// The human-readable alias.
    pub alias: String,
    /// The ID of the room being aliased.
    pub room_id: String,
    /// A list of homeserver domains that know about this alias.
    pub servers: Vec<String>,
}

#[derive(Debug, Queryable)]
pub struct RoomAlias {
    /// The human-readable alias.
    pub alias: String,
    /// The ID of the room being aliased.
    pub room_id: String,
    /// A list of homeserver domains that know about this alias.
    pub servers: Vec<String>,
    /// The time the room alias was created.
    pub created_at: PgTimestamp,
    /// The time the room alias was last modified.
    pub updated_at: PgTimestamp,
}

impl RoomAlias {
    /// Creates a new room alias in the database.
    pub fn create(connection: &PgConnection, new_room_alias: &NewRoomAlias)
    -> Result<RoomAlias, APIError> {
        insert(new_room_alias)
            .into(room_aliases::table)
            .get_result(connection)
            .map_err(APIError::from)
    }
}
