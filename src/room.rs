//! Matrix rooms.

use std::convert::TryInto;

use diesel::{Connection, ExecuteDsl, LoadDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use rand::{Rng, thread_rng};
use ruma_events::EventType;
use ruma_events::room::create::{CreateEvent, CreateEventContent};
use ruma_events::room::name::{NameEvent, NameEventContent};
use ruma_events::room::topic::{TopicEvent, TopicEventContent};

use error::APIError;
use event::{NewEvent, generate_event_id};
use room_alias::{NewRoomAlias, RoomAlias};
use schema::{events, rooms};

/// Options provided by the user to customize the room upon creation.
pub struct CreationOptions {
    /// An initial alias for the room.
    pub alias: Option<String>,
    /// Whehter or not the room should be federated.
    pub federate: bool,
    /// An initial name for the room.
    pub name: Option<String>,
    /// An initial topic for the room.
    pub topic: Option<String>,
}

/// A new Matrix room, not yet saved.
#[derive(Debug)]
#[insertable_into(rooms)]
pub struct NewRoom {
    /// The room's unique ID.
    pub id: String,
    /// The ID of the user creating the room.
    pub user_id: String,
    /// Whether or not the room is visible in the directory.
    pub public: bool,
}

/// A Matrix room.
#[derive(Debug, Queryable)]
pub struct Room {
    /// The room's unique ID.
    pub id: String,
    /// The ID of the user who created the room.
    pub user_id: String,
    /// Whether or not the room is visible in the directory.
    pub public: bool,
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

            if let Some(ref alias) = creation_options.alias {
                let new_room_alias = NewRoomAlias {
                    alias: alias.to_string(),
                    room_id: room.id.clone(),
                    servers: vec![homeserver_domain.to_string()],
                };

                RoomAlias::create(connection, &new_room_alias)?;
            }

            let new_create_event: NewEvent = CreateEvent {
                content: CreateEventContent {
                    creator: new_room.user_id.clone(),
                    federate: creation_options.federate,
                },
                event_id: generate_event_id(),
                event_type: EventType::RoomCreate,
                extra_content: (),
                prev_content: None,
                room_id: room.id.clone(),
                state_key: "".to_string(),
                unsigned: None,
                user_id: new_room.user_id.clone(),
            }.try_into()?;

            insert(&new_create_event)
                .into(events::table)
                .execute(connection)
                .map_err(APIError::from)?;

            if let Some(ref name) = creation_options.name {
                let new_name_event: NewEvent = NameEvent {
                    content: NameEventContent {
                        name: name.to_string(),
                    },
                    event_id: generate_event_id(),
                    event_type: EventType::RoomName,
                    extra_content: (),
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: new_room.user_id.clone(),
                }.try_into()?;

                insert(&new_name_event)
                    .into(events::table)
                    .execute(connection)
                    .map_err(APIError::from)?;
            }

            if let Some(ref topic) = creation_options.topic {
                let new_topic_event: NewEvent = TopicEvent {
                    content: TopicEventContent {
                        topic: topic.to_string(),
                    },
                    event_id: generate_event_id(),
                    event_type: EventType::RoomTopic,
                    extra_content: (),
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: new_room.user_id.clone(),
                }.try_into()?;

                insert(&new_topic_event)
                    .into(events::table)
                    .execute(connection)
                    .map_err(APIError::from)?;
            }

            Ok(room)
        }).map_err(APIError::from)
    }

    /// Generate a random room ID.
    pub fn generate_room_id() -> String {
        thread_rng().gen_ascii_chars().take(12).collect()
    }
}
