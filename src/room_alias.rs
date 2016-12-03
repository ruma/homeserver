//! Human-readable aliases for room IDs.

use std::convert::TryInto;

use diesel::{
    Connection,
    ExpressionMethods,
    FilterDsl,
    FindDsl,
    LoadDsl,
    ExecuteDsl,
    insert,
    delete,
};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::{Error as DieselError, DatabaseErrorKind};
use ruma_identifiers::{EventId, RoomAliasId, RoomId, UserId};
use ruma_events::room::aliases::{AliasesEvent, AliasesEventContent};
use ruma_events::EventType;

use error::ApiError;
use event::NewEvent;
use room::Room;
use schema::{events, room_aliases, rooms};

/// A new room alias, not yet saved.
#[derive(Debug)]
#[insertable_into(room_aliases)]
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
    pub fn create(connection: &PgConnection, homeserver_domain: &str, new_room_alias: &NewRoomAlias)
    -> Result<RoomAlias, ApiError> {
        connection.transaction(|| {
            let room_result = rooms::table
                .find(new_room_alias.room_id.to_string())
                .first::<Room>(&*connection)
                .map_err(|err| {
                   match err {
                       DieselError::NotFound => ApiError::bad_json(Some("Room not found.")),
                       _ => ApiError::from(err),
                   }
               });

            if room_result.is_err() {
                return Err(room_result.err().unwrap());
            }

            let aliases = RoomAlias::find_by_room_id(connection, &new_room_alias.room_id)?;
            let mut ids: Vec<RoomAliasId> = aliases.iter().map(|a| a.alias.clone()).collect();
            ids.push(new_room_alias.alias.clone());

            let new_room_alias_event: NewEvent = AliasesEvent {
                content: AliasesEventContent { aliases: ids },
                event_id: EventId::new(homeserver_domain)?,
                event_type: EventType::RoomAliases,
                extra_content: (),
                prev_content: None,
                room_id: new_room_alias.room_id.clone(),
                state_key: homeserver_domain.to_string(),
                unsigned: None,
                user_id: new_room_alias.user_id.clone(),
            }.try_into()?;

            insert(&new_room_alias_event)
                .into(events::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            insert(new_room_alias)
                .into(room_aliases::table)
                .get_result(connection)
                .map_err(|err| match err {
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)
                        => ApiError::alias_taken(None),
                    _ => ApiError::from(err),
                })
        }).map_err(ApiError::from)
    }

    /// Return the `RoomAlias` entry for given `RoomAliasId`.
    pub fn find_by_alias(connection: &PgConnection, alias: &RoomAliasId)
    -> Result<RoomAlias, ApiError> {
        room_aliases::table
            .find(alias)
            .get_result(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Return all aliases associated with the given `RoomId`.
    fn find_by_room_id(connection: &PgConnection, room_id: &RoomId)
    -> Result<Vec<RoomAlias>, ApiError> {
        let aliases: Vec<RoomAlias> = room_aliases::table
            .filter(room_aliases::room_id.eq(room_id))
            .get_results(connection)
            .map_err(ApiError::from)?;

        Ok(aliases)
    }

    /// Deletes a room alias in the database.
    pub fn delete(connection: &PgConnection, alias_id: &RoomAliasId, user_id: &UserId)
                  -> Result<usize, ApiError> {
        let alias = room_aliases::table
            .filter(room_aliases::alias.eq(alias_id.to_string()))
            .filter(room_aliases::user_id.eq(user_id.to_string()));

        delete(alias)
            .execute(connection)
            .map_err(ApiError::from)
    }
}
