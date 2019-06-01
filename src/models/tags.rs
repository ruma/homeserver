//! Matrix room tags.
use std::collections::HashMap;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::result::Error as DieselError;
use ruma_events::tag::TagInfo;
use ruma_identifiers::{RoomId, UserId};
use serde_json::de::from_str;

use crate::error::ApiError;
use crate::models::room::Room;
use crate::schema::{rooms, room_tags};

/// A new Matrix room tag, not yet saved.
#[derive(Debug, Clone, Insertable)]
#[table_name = "room_tags"]
pub struct NewRoomTag {
    /// The user's ID.
    pub user_id: UserId,
    /// The room's ID.
    pub room_id: RoomId,
    /// Tag
    pub tag: String,
    /// Json content
    pub content: String,
}

/// A Matrix room tag.
#[derive(Debug, Clone, AsChangeset, Identifiable, Queryable)]
#[table_name = "room_tags"]
pub struct RoomTag {
    /// Entry ID
    pub id: i64,
    /// The user's ID.
    pub user_id: UserId,
    /// The room's ID.
    pub room_id: RoomId,
    /// Tag
    pub tag: String,
    /// Json content
    pub content: String,
}


impl RoomTag {
    /// Return `RoomTag`'s for given `UserId` and `RoomId`.
    pub fn find(
        connection: &PgConnection,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<HashMap<String, TagInfo>, ApiError> {
        rooms::table.find(room_id.to_string()).first::<Room>(&*connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found("The given room_id does not correspond to tags".to_string()),
                _ => ApiError::from(err),
            })?;
        let tags: Vec<RoomTag> = room_tags::table
            .filter(room_tags::room_id.eq(room_id))
            .filter(room_tags::user_id.eq(user_id))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found("The given user_id and room_id does not correspond to tags".to_string()),
                _ => ApiError::from(err),
            })?;

        let mut map: HashMap<String, TagInfo> = HashMap::new();
        for tag in tags {
            let info = from_str(&tag.content).map_err(ApiError::from)?;
            map.insert(tag.tag, info);
        }

        Ok(map)
    }

    /// Return `RoomTag` for given `UserId`, `RoomId` and `tag`.
    pub fn first(
        connection: &PgConnection,
        user_id: UserId,
        room_id: RoomId,
        tag: String,
    ) -> Result<Option<RoomTag>, ApiError> {
        let tag = room_tags::table
            .filter(room_tags::room_id.eq(room_id))
            .filter(room_tags::user_id.eq(user_id))
            .filter(room_tags::tag.eq(tag))
            .first(connection);

        match tag {
            Ok(tag) => Ok(Some(tag)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Update or Insert a `RoomTag`.
    pub fn upsert(
        connection: &PgConnection,
        user_id: UserId,
        room_id: RoomId,
        tag: String,
        content: String,
    ) -> Result<(), ApiError> {
        let entry = RoomTag::first(connection, user_id.clone(), room_id.clone(), tag.clone())?;

        match entry {
            Some(mut entry) => entry.update(connection, content),
            None => RoomTag::create(connection, user_id, room_id, tag, content)
        }
    }

    /// Create a `RoomTag`.
    pub fn create(
        connection: &PgConnection,
        user_id: UserId,
        room_id: RoomId,
        tag: String,
        content: String,
    ) -> Result<(), ApiError> {
        connection.transaction(|| {
            rooms::table.find(room_id.to_string()).first::<Room>(&*connection)
                .map_err(|err| match err {
                    DieselError::NotFound => ApiError::not_found("The given room_id does not correspond to a room".to_string()),
                    _ => ApiError::from(err),
                })?;
            let new_room_tag = NewRoomTag {
                user_id,
                room_id,
                tag,
                content,
            };
            diesel::insert_into(room_tags::table)
                .values(&new_room_tag)
                .execute(connection)
                .map_err(ApiError::from)
        })?;
        Ok(())
    }

    /// Update a `RoomTag`.
    pub fn update(&mut self, connection: &PgConnection, content: String) -> Result<(), ApiError> {
        self.content = content;
        self.save_changes::<RoomTag>(connection)
            .map_err(ApiError::from)?;
        Ok(())
    }

    /// Delete a `RoomTag`.
    pub fn delete(
        connection: &PgConnection,
        user_id: UserId,
        room_id: RoomId,
        tag: String,
    ) -> Result<(), ApiError> {
        let tag = room_tags::table
            .filter(room_tags::room_id.eq(room_id))
            .filter(room_tags::user_id.eq(user_id))
            .filter(room_tags::tag.eq(tag));
        tag.clone().first::<RoomTag>(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found("The given room_id does not correspond to a tag".to_string()),
                _ => ApiError::from(err),
            })?;
        diesel::delete(tag)
            .execute(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found("The given user_id and room_id does not correspond to a tag".to_string()),
                _ => ApiError::from(err),
            })?;
        Ok(())
    }
}
