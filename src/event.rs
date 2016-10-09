//! Matrix events.

use std::convert::{TryInto, TryFrom};

use diesel::{ExpressionMethods, FilterDsl, LoadDsl};
use diesel::result::Error as DieselError;
use diesel::pg::data_types::PgTimestamp;
use diesel::pg::PgConnection;
use ruma_events::{RoomEvent, StateEvent, EventType};
use ruma_events::room::join_rules::{JoinRulesEvent};
use ruma_events::room::member::MemberEvent;
use ruma_identifiers::{EventId, RoomId, UserId};
use serde::{Serialize, Deserialize};
use serde_json::{Error as SerdeJsonError, from_str, to_string};

use error::ApiError;
use schema::events;

/// A new event, not yet saved.
#[derive(Debug)]
#[insertable_into(events)]
pub struct NewEvent {
    /// The type of the event, e.g. *m.room.create*.
    pub event_type: String,
    /// Extra key-value pairs to be mixed into the top-level JSON representation of the event.
    pub extra_content: Option<String>,
    /// The unique event ID.
    pub id: EventId,
    /// JSON of the event's content.
    pub content: String,
    /// The room the event was sent in.
    pub room_id: RoomId,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
    /// The user who sent the event.
    pub user_id: UserId,
}

/// A Matrix event.
#[derive(Debug, Queryable)]
pub struct Event {
    /// The unique event ID.
    pub id: EventId,
    /// The depth of the event within its room, with the first event in the room being 1.
    pub ordering: i64,
    /// The room the event was sent in.
    pub room_id: RoomId,
    /// The user who sent the event.
    pub user_id: UserId,
    /// The type of the event, e.g. *m.room.create*.
    pub event_type: String,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
    /// JSON of the event's content.
    pub content: String,
    /// Extra key-value pairs to be mixed into the top-level JSON representation of the event.
    pub extra_content: Option<String>,
    /// The time the event was created.
    pub created_at: PgTimestamp,
}

impl Event {
    /// Return room join rules for given `room_id`.
    pub fn find_room_join_rules_by_room_id(connection: &PgConnection, room_id: RoomId)
        -> Result<JoinRulesEvent, ApiError>
    {
        let event: Event = events::table
            .filter(events::event_type.eq((&EventType::RoomJoinRules).to_string()))
            .filter(events::room_id.eq(room_id))
            .first(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;
        TryInto::try_into(event).map_err(ApiError::from)
    }
}

impl<C> TryFrom<RoomEvent<C, ()>> for NewEvent where C: Deserialize + Serialize {
    fn try_from(event: RoomEvent<C, ()>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: None,
            id: event.event_id,
            room_id: event.room_id,
            state_key: None,
            user_id: event.user_id,
        })
    }
}

impl<C, E> TryFrom<RoomEvent<C, E>> for NewEvent
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_from(event: RoomEvent<C, E>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: Some(to_string(&event.extra_content)?),
            id: event.event_id,
            room_id: event.room_id,
            state_key: None,
            user_id: event.user_id,
        })
    }
}

impl<C> TryFrom<StateEvent<C, ()>> for NewEvent where C: Deserialize + Serialize {
    fn try_from(event: StateEvent<C, ()>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: None,
            id: event.event_id,
            room_id: event.room_id,
            state_key: Some(event.state_key),
            user_id: event.user_id,
        })
    }
}

impl<C, E> TryFrom<StateEvent<C, E>> for NewEvent
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_from(event: StateEvent<C, E>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: Some(to_string(&event.extra_content)?),
            id: event.event_id,
            room_id: event.room_id,
            state_key: Some(event.state_key),
            user_id: event.user_id,
        })
    }
}

impl<C> TryInto<RoomEvent<C, ()>> for Event where C: Deserialize + Serialize {
    fn try_into(self) -> Result<RoomEvent<C, ()>, Self::Err> {
        Ok(RoomEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            extra_content: (),
            event_type: from_str(&self.event_type)?,
            room_id: self.room_id,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl<C, E> TryInto<RoomEvent<C, E>> for Event
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_into(self) -> Result<RoomEvent<C, E>, Self::Err> {
        Ok(RoomEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            extra_content: from_str(&self.extra_content.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            event_type: from_str(&self.event_type)?,
            room_id: self.room_id,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl<C> TryInto<StateEvent<C, ()>> for Event where C: Deserialize + Serialize {
    default fn try_into(self) -> Result<StateEvent<C, ()>, Self::Err> {
        Ok(StateEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            extra_content: (),
            event_type: from_str(&self.event_type)?,
            prev_content: None,
            room_id: self.room_id,
            state_key: from_str(&self.state_key.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl<C, E> TryInto<StateEvent<C, E>> for Event
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_into(self) -> Result<StateEvent<C, E>, Self::Err> {
        Ok(StateEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            extra_content: from_str(&self.extra_content.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            event_type: from_str(&self.event_type)?,
            prev_content: None,
            room_id: self.room_id,
            state_key: from_str(&self.state_key.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl TryInto<JoinRulesEvent> for Event {
    fn try_into(self) -> Result<JoinRulesEvent, Self::Err> {
        Ok(JoinRulesEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            extra_content: (),
            event_type: EventType::RoomJoinRules,
            prev_content: None,
            room_id: self.room_id,
            state_key: "".to_string(),
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl TryInto<MemberEvent> for Event {
    fn try_into(self) -> Result<MemberEvent, Self::Err> {
        Ok(MemberEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            prev_content: None,
            extra_content: from_str(&self.extra_content.unwrap())?,
            state_key: "".to_string(),
            event_type: EventType::RoomMember,
            room_id: self.room_id,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}