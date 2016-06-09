//! Matrix rooms.

use diesel::{Connection, LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use rand::{Rng, thread_rng};

use error::APIError;
use room_alias::{NewRoomAlias, RoomAlias};
use schema::rooms;

/// Options provided by the user to customize the room upon creation.
pub struct CreationOptions {
    /// An initial alias for the room.
    pub room_alias_name: Option<String>,
}

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
    pub fn create(
        connection: &PgConnection,
        new_room: &NewRoom,
        homeserver_domain: &str,
        creation_options: &CreationOptions,
    ) -> Result<Room, APIError> {
        connection.transaction::<Room, APIError, _>(|| {
            let room: Room = insert(new_room)
                .into(rooms::table)
                .get_result(connection)
                .map_err(APIError::from)?;

            if let Some(ref alias) = creation_options.room_alias_name {
                let new_room_alias = NewRoomAlias {
                    alias: alias.to_string(),
                    room_id: room.id.clone(),
                    servers: vec![homeserver_domain.to_string()],
                };

                RoomAlias::create(connection, &new_room_alias)?;
            }

            Ok(room)
        }).map_err(APIError::from)
    }

    /// Generate a random room ID.
    pub fn generate_room_id() -> String {
        thread_rng().gen_ascii_chars().take(12).collect()
    }
}
