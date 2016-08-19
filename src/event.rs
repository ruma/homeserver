//! Matrix events.

use std::convert::{TryInto, TryFrom};

use diesel::pg::data_types::PgTimestamp;
use ruma_events::{RoomEvent, StateEvent};
use serde::{Serialize, Deserialize};
use serde_json::{Error as SerdeJsonError, from_str, to_string};

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
    pub id: String,
    /// JSON of the event's content.
    pub content: String,
    /// The room the event was sent in.
    pub room_id: String,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
    /// The user who sent the event.
    pub user_id: String,
}

/// A Matrix event.
#[derive(Debug, Queryable)]
pub struct Event {
    /// The depth of the event within its room, with the first event in the room being 1.
    pub ordering: i64,
    /// The type of the event, e.g. *m.room.create*.
    pub event_type: String,
    /// Extra key-value pairs to be mixed into the top-level JSON representation of the event.
    pub extra_content: Option<String>,
    /// The unique event ID.
    pub id: String,
    /// JSON of the event's content.
    pub content: String,
    /// The room the event was sent in.
    pub room_id: String,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
    /// The user who sent the event.
    pub user_id: String,
    /// The time the event was created.
    pub created_at: PgTimestamp,
}

impl<C> TryFrom<RoomEvent<C, ()>> for NewEvent where C: Deserialize + Serialize {
    fn try_from(event: RoomEvent<C, ()>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: None,
            id: event.event_id.to_string(),
            room_id: event.room_id.to_string(),
            state_key: None,
            user_id: event.user_id.to_string(),
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
            id: event.event_id.to_string(),
            room_id: event.room_id.to_string(),
            state_key: None,
            user_id: event.user_id.to_string(),
        })
    }
}

impl<C> TryFrom<StateEvent<C, ()>> for NewEvent where C: Deserialize + Serialize {
    fn try_from(event: StateEvent<C, ()>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            content: to_string(&event.content)?,
            event_type: event.event_type.to_string(),
            extra_content: None,
            id: event.event_id.to_string(),
            room_id: event.room_id.to_string(),
            state_key: Some(event.state_key),
            user_id: event.user_id.to_string(),
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
            id: event.event_id.to_string(),
            room_id: event.room_id.to_string(),
            state_key: Some(event.state_key),
            user_id: event.user_id.to_string(),
        })
    }
}

impl<C> TryInto<RoomEvent<C, ()>> for Event where C: Deserialize + Serialize {
    fn try_into(self) -> Result<RoomEvent<C, ()>, Self::Err> {
        Ok(RoomEvent {
            content: from_str(&self.content)?,
            event_id: from_str(&self.id)?,
            extra_content: (),
            event_type: from_str(&self.event_type)?,
            room_id: from_str(&self.room_id)?,
            unsigned: None,
            user_id: from_str(&self.user_id)?,
        })
    }
}

impl<C, E> TryInto<RoomEvent<C, E>> for Event
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_into(self) -> Result<RoomEvent<C, E>, Self::Err> {
        Ok(RoomEvent {
            content: from_str(&self.content)?,
            event_id: from_str(&self.id)?,
            extra_content: from_str(&self.extra_content.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            event_type: from_str(&self.event_type)?,
            room_id: from_str(&self.room_id)?,
            unsigned: None,
            user_id: from_str(&self.user_id)?,
        })
    }
}

impl<C> TryInto<StateEvent<C, ()>> for Event where C: Deserialize + Serialize {
    fn try_into(self) -> Result<StateEvent<C, ()>, Self::Err> {
        Ok(StateEvent {
            content: from_str(&self.content)?,
            event_id: from_str(&self.id)?,
            extra_content: (),
            event_type: from_str(&self.event_type)?,
            prev_content: None,
            room_id: from_str(&self.room_id)?,
            state_key: from_str(&self.state_key.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            unsigned: None,
            user_id: from_str(&self.user_id)?,
        })
    }
}

impl<C, E> TryInto<StateEvent<C, E>> for Event
where C: Deserialize + Serialize, E: Deserialize + Serialize {
    type Err = SerdeJsonError;

    default fn try_into(self) -> Result<StateEvent<C, E>, Self::Err> {
        Ok(StateEvent {
            content: from_str(&self.content)?,
            event_id: from_str(&self.id)?,
            extra_content: from_str(&self.extra_content.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            event_type: from_str(&self.event_type)?,
            prev_content: None,
            room_id: from_str(&self.room_id)?,
            state_key: from_str(&self.state_key.expect(
                "failed to deserialize extra event content from the DB record"
            ))?,
            unsigned: None,
            user_id: from_str(&self.user_id)?,
        })
    }
}
