//! Matrix events.

use std::convert::TryFrom;

use ruma_events::StateEvent;
use serde::{Serialize, Deserialize};
use serde_json::{Error as SerdeJsonError, to_string};

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

impl<C> TryFrom<StateEvent<C, ()>> for NewEvent where C: Deserialize + Serialize {
    fn try_from(event: StateEvent<C, ()>) -> Result<Self, Self::Err> {
        Ok(NewEvent {
            event_type: event.event_type.to_string(),
            extra_content: None,
            id: event.event_id.to_string(),
            content: to_string(&event.content)?,
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
            event_type: event.event_type.to_string(),
            extra_content: Some(to_string(&event.extra_content)?),
            id: event.event_id.to_string(),
            content: to_string(&event.content)?,
            room_id: event.room_id.to_string(),
            state_key: Some(event.state_key),
            user_id: event.user_id.to_string(),
        })
    }
}
