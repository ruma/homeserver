//! Matrix rooms.

use diesel::{LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use rand::{Rng, thread_rng};

use error::APIError;
use schema::rooms;

/// A new Matrix room, not yet saved.
#[derive(Debug)]
#[insertable_into(rooms)]
pub struct NewRoom {
    /// The room's unique ID.
    pub id: String,
    /// The ID of the user creating the room.
    pub user_id: String,
}

/// A Matrix room.
#[derive(Debug, Queryable)]
pub struct Room {
    /// The room's unique ID.
    pub id: String,
    /// The ID of the user who created the room.
    pub user_id: String,
    /// The time the room was created.
    pub created_at: PgTimestamp,
}

impl Room {
    /// Creates a new room in the database.
    pub fn create(connection: &PgConnection, new_room: &NewRoom) -> Result<Room, APIError> {
        let room: Room = insert(new_room)
            .into(rooms::table)
            .get_result(connection)
            .map_err(APIError::from)?;

        Ok(room)
    }

    /// Generate a random room ID.
    pub fn generate_room_id() -> String {
        thread_rng().gen_ascii_chars().take(12).collect()
    }
}
