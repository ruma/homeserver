//! Human-readable aliases for room IDs.

use diesel::{ExpressionMethods, FilterDsl, LoadDsl, ExecuteDsl, insert, delete};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::{Error as DieselError, DatabaseErrorKind};
use ruma_identifiers::RoomId;

use error::ApiError;
use schema::room_aliases;

/// A new room alias, not yet saved.
#[derive(Debug)]
#[insertable_into(room_aliases)]
pub struct NewRoomAlias {
    /// The human-readable alias.
    pub alias: String,
    /// The ID of the room being aliased.
    pub room_id: RoomId,
    /// A list of homeserver domains that know about this alias.
    pub servers: Vec<String>,
}

/// A human-readable alias for a room ID.
#[derive(Debug, Queryable)]
pub struct RoomAlias {
    /// The human-readable alias.
    pub alias: String,
    /// The ID of the room being aliased.
    pub room_id: RoomId,
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
    -> Result<RoomAlias, ApiError> {
        insert(new_room_alias)
            .into(room_aliases::table)
            .get_result(connection)
            .map_err(|err| match err {
                DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)
                    => ApiError::alias_taken(None),
                _ => ApiError::from(err),
            })
    }

    /// Return room ID for given room alias.
    pub fn find_by_alias(connection: &PgConnection, alias: &str)
    -> Result<RoomAlias, ApiError> {
        room_aliases::table
            .filter(room_aliases::alias.eq(alias))
            .first(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Deletes a room alias in the database.
    pub fn delete(connection: &PgConnection, room_alias: &str)
                  -> Result<usize, ApiError> {
        let thing = room_aliases::table.filter(room_aliases::alias.eq(room_alias));

        delete(thing)
            .execute(connection)
            .map_err(ApiError::from)
    }
}
