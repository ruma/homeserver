//! Human-readable aliases for room IDs.

use std::convert::TryInto;

use diesel::pg::data_types::PgTimestamp;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use ruma_events::room::aliases::{AliasesEvent, AliasesEventContent};
use ruma_events::EventType;
use ruma_identifiers::{EventId, RoomAliasId, RoomId, UserId};

use crate::error::ApiError;
use crate::models::event::NewEvent;
use crate::models::room::Room;
use crate::schema::{events, room_aliases};

/// A new room alias, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "room_aliases"]
pub struct NewRoomAlias {
    /// The human-readable alias.
    pub alias: RoomAliasId,
    /// The ID of the room being aliased.
    pub room_id: RoomId,
    /// The ID of the user creating the alias.
    pub user_id: UserId,
    /// A list of homeserver domains that know about this alias.
    pub servers: Vec<String>,
}

/// A human-readable alias for a room ID.
#[derive(Debug, Queryable)]
pub struct RoomAlias {
    /// The human-readable alias.
    pub alias: RoomAliasId,
    /// The ID of the room being aliased.
    pub room_id: RoomId,
    /// The ID of the user that created the alias.
    pub user_id: UserId,
    /// A list of homeserver domains that know about this alias.
    pub servers: Vec<String>,
    /// The time the room alias was created.
    pub created_at: PgTimestamp,
    /// The time the room alias was last modified.
    pub updated_at: PgTimestamp,
}

impl RoomAlias {
    /// Creates a new room alias in the database.
    pub fn create(
        connection: &PgConnection,
        homeserver_domain: &str,
        new_room_alias: &NewRoomAlias,
    ) -> Result<RoomAlias, ApiError> {
        connection
            .transaction(|| {
                if Room::find(connection, &new_room_alias.room_id)?.is_none() {
                    return Err(ApiError::bad_json("Room not found".to_string()));
                }

                let aliases = RoomAlias::find_by_room_id(connection, &new_room_alias.room_id)?;
                let mut ids: Vec<RoomAliasId> = aliases.iter().map(|a| a.alias.clone()).collect();
                ids.push(new_room_alias.alias.clone());

                let new_room_alias_event: NewEvent = AliasesEvent {
                    content: AliasesEventContent { aliases: ids },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomAliases,
                    origin_server_ts: 0,
                    prev_content: None,
                    room_id: Some(new_room_alias.room_id.clone()),
                    sender: new_room_alias.user_id.clone(),
                    state_key: homeserver_domain.to_string(),
                    unsigned: None,
                }
                .try_into()?;

                diesel::insert_into(events::table)
                    .values(&new_room_alias_event)
                    .execute(connection)
                    .map_err(ApiError::from)?;

                diesel::insert_into(room_aliases::table)
                    .values(new_room_alias)
                    .get_result(connection)
                    .map_err(|err| match err {
                        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                            ApiError::alias_taken(None)
                        }
                        _ => ApiError::from(err),
                    })
            })
            .map_err(ApiError::from)
    }

    /// Return the `RoomAlias` entry for given `RoomAliasId`.
    pub fn find_by_alias(
        connection: &PgConnection,
        alias: &RoomAliasId,
    ) -> Result<RoomAlias, ApiError> {
        room_aliases::table
            .find(alias)
            .get_result(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Return all aliases associated with the given `RoomId`.
    fn find_by_room_id(
        connection: &PgConnection,
        room_id: &RoomId,
    ) -> Result<Vec<RoomAlias>, ApiError> {
        let aliases: Vec<RoomAlias> = room_aliases::table
            .filter(room_aliases::room_id.eq(room_id))
            .get_results(connection)
            .map_err(ApiError::from)?;

        Ok(aliases)
    }

    /// Deletes a room alias in the database.
    pub fn delete(
        connection: &PgConnection,
        alias_id: &RoomAliasId,
        user_id: &UserId,
    ) -> Result<usize, ApiError> {
        let alias = room_aliases::table
            .filter(room_aliases::alias.eq(alias_id.to_string()))
            .filter(room_aliases::user_id.eq(user_id.to_string()));

        diesel::delete(alias)
            .execute(connection)
            .map_err(ApiError::from)
    }
}
