//! Matrix filter.

use std::fmt::{Formatter, Result as FmtResult};

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use ruma_identifiers::{RoomId, UserId};
use serde::de::{Error as SerdeError, Unexpected, Visitor};
use serde::{Deserializer, Serializer};

use crate::error::ApiError;
use crate::schema::filters;

/// Defines the default format of `Filter` for `account_data` and `presence`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// A list of event types to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub not_types: Vec<String>,
    /// The maximum number of events to return.
    pub limit: usize,
    /// A list of senders IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub senders: Vec<UserId>,
    /// A list of event types to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub types: Vec<String>,
    /// A list of sender IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub not_senders: Vec<UserId>,
}

/// Defines the default format of a `RoomEventFilter`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEventFilter {
    /// A list of event types to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub not_types: Vec<String>,
    /// A list of event types to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub types: Vec<String>,
    /// A list of room IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub not_rooms: Vec<RoomId>,
    /// A list of room IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub rooms: Vec<RoomId>,
    /// The maximum number of events to return.
    pub limit: usize,
    /// A list of sender IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub not_senders: Vec<UserId>,
    /// A list of senders IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub senders: Vec<UserId>,
}

fn default_include_leave() -> bool {
    false
}

fn default_vec_room_id() -> Vec<RoomId> {
    Vec::new()
}

fn default_vec_user_id() -> Vec<UserId> {
    Vec::new()
}

fn is_false(test: &bool) -> bool {
    !test
}

/// `RoomFilter`'s to be applied to room data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFilter {
    /// Include rooms that the user has left in the sync, default false
    #[serde(default = "default_include_leave")]
    #[serde(skip_serializing_if = "is_false")]
    pub include_leave: bool,
    /// The per user account data to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_data: Option<RoomEventFilter>,
    /// The message and state update events to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<RoomEventFilter>,
    /// The events that aren't recorded in the room history, e.g. typing and receipts, to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<RoomEventFilter>,
    /// The state events to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<RoomEventFilter>,
    /// A list of room IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub not_rooms: Vec<RoomId>,
    /// A list of room IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub rooms: Vec<RoomId>,
}

/// Predefined `EventFormat` types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventFormat {
    /// 'client' will return the events in a format suitable for clients.
    Client,
    /// 'federation' will return the raw event as received over federation.
    Federation,
}

impl ::serde::Serialize for EventFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match *self {
            EventFormat::Client => "client",
            EventFormat::Federation => "federation",
        })
    }
}

impl<'de> ::serde::Deserialize<'de> for EventFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EventFormatVisitor;

        impl<'de> Visitor<'de> for EventFormatVisitor {
            type Value = EventFormat;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                write!(formatter, "an event format")
            }

            fn visit_str<E>(self, value: &str) -> Result<EventFormat, E>
            where
                E: SerdeError,
            {
                match value {
                    "client" => Ok(EventFormat::Client),
                    "federation" => Ok(EventFormat::Federation),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(EventFormatVisitor)
    }
}

/// `ContentFilter` contains all information to filter request like `sync`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilter {
    /// Filters to be applied to room data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<RoomFilter>,
    /// The presence updates to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<EventFilter>,
    /// The user account data that isn't associated with rooms to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_data: Option<EventFilter>,
    /// The format to use for events. 'client' will return the events in a format suitable for clients. 'federation' will return the raw event as received over federation. The default is 'client'. One of: "client", "federation"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_format: Option<EventFormat>,
    /// List of event fields to include. If this list is absent then all fields are included. The entries may include '.' charaters to indicate sub-fields. So ['content.body'] will include the 'body' field of the 'content' object. A literal '.' character in a field name may be escaped using a '\'. A server may include more fields than were requested.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub event_fields: Vec<String>,
}

/// A new Matrix filter, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "filters"]
pub struct NewFilter {
    /// The user's ID.
    pub user_id: UserId,
    /// The contents.
    pub content: String,
}

/// A new Matrix filter.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "filters"]
pub struct Filter {
    /// Entry ID
    pub id: i64,
    /// The user's ID.
    pub user_id: UserId,
    /// The contents.
    pub content: String,
}

impl Filter {
    /// Creates a new `Filter`
    pub fn create(
        connection: &PgConnection,
        user_id: UserId,
        content: String,
    ) -> Result<i64, ApiError> {
        let new_filter = NewFilter { user_id, content };

        let filter: Filter = diesel::insert_into(filters::table)
            .values(&new_filter)
            .get_result(connection)
            .map_err(ApiError::from)?;
        Ok(filter.id)
    }

    /// Return `Filter`'s for given `UserId` and `id`.
    pub fn find(connection: &PgConnection, user_id: UserId, id: i64) -> Result<Filter, ApiError> {
        let filter = filters::table
            .filter(filters::id.eq(id))
            .filter(filters::user_id.eq(user_id))
            .first(connection);

        match filter {
            Ok(filter) => Ok(filter),
            Err(DieselError::NotFound) => Err(ApiError::not_found("".to_string())),
            Err(err) => Err(ApiError::from(err)),
        }
    }
}
