//! Matrix events.

use std::convert::{TryInto, TryFrom};

use diesel::prelude::*;
use diesel::dsl::{any, max};
use diesel::result::Error as DieselError;
use diesel::pg::data_types::PgTimestamp;
use diesel::pg::PgConnection;
use ruma_events::{
    CustomRoomEvent,
    CustomStateEvent,
    Event as RumaEventsEvent,
    EventType,
    RoomEvent,
    StateEvent as RumaStateEventTrait,
};
use ruma_events::call::answer::AnswerEvent;
use ruma_events::call::candidates::CandidatesEvent;
use ruma_events::call::hangup::HangupEvent;
use ruma_events::call::invite::InviteEvent;
use ruma_events::collections::all::StateEvent;
use ruma_events::room::aliases::AliasesEvent;
use ruma_events::room::avatar::AvatarEvent;
use ruma_events::room::canonical_alias::CanonicalAliasEvent;
use ruma_events::room::create::CreateEvent;
use ruma_events::room::guest_access::GuestAccessEvent;
use ruma_events::room::history_visibility::HistoryVisibilityEvent;
use ruma_events::room::join_rules::JoinRulesEvent;
use ruma_events::room::member::MemberEvent;
use ruma_events::room::message::MessageEvent;
use ruma_events::room::name::NameEvent;
use ruma_events::room::power_levels::PowerLevelsEvent;
use ruma_events::room::third_party_invite::ThirdPartyInviteEvent;
use ruma_events::room::topic::TopicEvent;
use ruma_events::stripped::{
    StrippedRoomAliases,
    StrippedRoomAvatar,
    StrippedRoomCanonicalAlias,
    StrippedRoomCreate,
    StrippedRoomGuestAccess,
    StrippedRoomHistoryVisibility,
    StrippedRoomJoinRules,
    StrippedRoomMember,
    StrippedRoomName,
    StrippedRoomPowerLevels,
    StrippedRoomThirdPartyInvite,
    StrippedRoomTopic,
    StrippedState,
};
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{from_str, to_string};

use crate::error::ApiError;
use crate::schema::events;

const STATE_EVENTS: [EventType; 12] = [
    EventType::RoomAliases,
    EventType::RoomAvatar,
    EventType::RoomCanonicalAlias,
    EventType::RoomCreate,
    EventType::RoomGuestAccess,
    EventType::RoomHistoryVisibility,
    EventType::RoomJoinRules,
    EventType::RoomMember,
    EventType::RoomName,
    EventType::RoomPowerLevels,
    EventType::RoomThirdPartyInvite,
    EventType::RoomTopic,
];

/// A new event, not yet saved.
#[derive(Debug, Clone, Insertable)]
#[table_name = "events"]
pub struct NewEvent {
    /// The type of the event, e.g. *m.room.create*.
    pub event_type: String,
    /// The unique event ID.
    pub id: EventId,
    /// JSON of the event's content.
    pub content: String,
    /// The room the event was sent in.
    pub room_id: Option<RoomId>,
    /// The user who sent the event.
    pub sender: UserId,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
}

/// A Matrix event.
#[derive(Clone, Debug, Queryable)]
pub struct Event {
    /// The unique event ID.
    pub id: EventId,
    /// The depth of the event within its room, with the first event in the room being 1.
    pub ordering: i64,
    /// The room the event was sent in.
    pub room_id: Option<RoomId>,
    /// The user who sent the event.
    pub sender: UserId,
    /// The type of the event, e.g. *m.room.create*.
    pub event_type: String,
    /// An event subtype that determines whether or not the event will overwrite a previous one.
    pub state_key: Option<String>,
    /// JSON of the event's content.
    pub content: String,
    /// The time the event was created.
    pub created_at: PgTimestamp,
}

impl Event {
    /// Return room join rules for given `room_id`.
    pub fn find_room_join_rules_by_room_id(connection: &PgConnection, room_id: RoomId)
        -> Result<JoinRulesEvent, ApiError>
    {
        let event: Event = events::table
            .filter(events::event_type.eq(EventType::RoomJoinRules.to_string()))
            .filter(events::room_id.eq(room_id))
            .order(events::ordering.desc())
            .first(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;
        TryInto::try_into(event).map_err(ApiError::from)
    }

    /// Return all `RoomEvent`'s for a `RoomId` after a specific point in time.
    pub fn find_room_events(connection: &PgConnection, room_id: &RoomId, since: i64) -> Result<Vec<Event>, ApiError> {
        events::table
            .filter(events::event_type.like("m.room.%"))
            .filter(events::ordering.gt(since))
            .filter(events::room_id.eq(room_id))
            .order(events::ordering.asc())
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Return all `RoomEvent`'s for a `RoomId` up to a specific point in time.
    pub fn find_room_events_until(
        connection: &PgConnection,
        room_id: &RoomId,
        until: &i64
    ) -> Result<Vec<Event>, ApiError> {
        events::table
            .filter(events::event_type.like("m.room.%"))
            .filter(events::ordering.lt(until))
            .filter(events::room_id.eq(room_id))
            .order(events::ordering.asc())
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Look up an event given its `EventId`.
    pub fn find(connection: &PgConnection, event_id: &EventId) -> Result<Option<Event>, ApiError> {
        match events::table.find(event_id).first(connection) {
            Ok(event) => Ok(Some(event)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Return the room's state before a specified event.
    pub fn get_room_state_events_until(
        connection: &PgConnection,
        room_id: &RoomId,
        until: &Event,
    ) -> Result<Vec<Event>, ApiError> {
        let state_events: Vec<String> = STATE_EVENTS.iter()
            .map(EventType::to_string)
            .collect();

        let ordering = events::table
            .select(max(events::ordering))
            .filter(events::room_id.eq(room_id))
            .filter(events::event_type.eq(any(state_events)))
            .filter(events::ordering.lt(until.ordering))
            .group_by(events::event_type);

        events::table
            .filter(events::ordering.nullable().eq(any(ordering)))
            .get_results(connection)
            .map_err(ApiError::from)
    }

    /// Returns the room's current state.
    pub fn get_room_full_state(connection: &PgConnection, room_id: &RoomId) -> Result<Vec<Event>, ApiError> {
        Event::get_room_state_events_since(connection, room_id, -1)
    }

    /// Return the state changes in a room after a specific point in time.
    pub fn get_room_state_events_since(
        connection: &PgConnection,
        room_id: &RoomId,
        since: i64
    ) -> Result<Vec<Event>, ApiError> {
        let state_events: Vec<String> = STATE_EVENTS.iter()
            .map(EventType::to_string)
            .collect();

        let ordering = events::table
            .select(max(events::ordering))
            .filter(events::room_id.eq(room_id))
            .filter(events::event_type.eq(any(state_events)))
            .filter(events::ordering.gt(since))
            .group_by(events::event_type);

        events::table
            .filter(events::ordering.nullable().eq(any(ordering)))
            .get_results(connection)
            .map_err(ApiError::from)
    }
}


macro_rules! impl_try_from_room_event_for_new_event {
    ($ty:ty) => {
        impl TryFrom<$ty> for NewEvent {
            type Error = ApiError;

            fn try_from(event: $ty) -> Result<Self, Self::Error> {
                Ok(NewEvent {
                    content: to_string(event.content()).map_err(ApiError::from)?,
                    event_type: event.event_type().to_string(),
                    id: event.event_id().clone(),
                    room_id: event.room_id().map(|room_id| room_id.clone()),
                    sender: event.sender().clone(),
                    state_key: None,
                })
            }
        }
    }
}

macro_rules! impl_try_from_state_event_for_new_event {
    ($ty:ty) => {
        impl TryFrom<$ty> for NewEvent {
            type Error = ApiError;

            fn try_from(event: $ty) -> Result<Self, Self::Error> {
                Ok(NewEvent {
                    content: to_string(event.content()).map_err(ApiError::from)?,
                    event_type: event.event_type().to_string(),
                    id: event.event_id().clone(),
                    room_id: event.room_id().map(|room_id| room_id.clone()),
                    sender: event.sender().clone(),
                    state_key: Some(event.state_key().to_string()),
                })
            }
        }
    }
}

macro_rules! impl_try_into_room_event_for_event {
    ($ty:ident) => {
        impl TryInto<$ty> for Event {
            type Error = ApiError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                Ok($ty {
                    content: from_str(&self.content).map_err(ApiError::from)?,
                    event_id: self.id,
                    event_type: EventType::from(self.event_type.as_ref()),
                    // FIXME: This is a dummy value just to satisfy event types' new schema.
                    // The real value should come from the database record's created_at timestamp,
                    // but it's unclear exactly how.
                    //
                    // See https://github.com/matrix-org/matrix-doc/issues/2064
                    origin_server_ts: 0,
                    room_id: self.room_id,
                    sender: self.sender,
                    unsigned: None,
                })
            }
        }
    };
}

macro_rules! impl_try_into_state_event_for_event {
    ($ty:ident) => {
        impl TryInto<$ty> for Event {
            type Error = ApiError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                Ok($ty {
                    content: from_str(&self.content).map_err(ApiError::from)?,
                    event_id: self.id,
                    event_type: EventType::from(self.event_type.as_ref()),
                    // FIXME: This is a dummy value just to satisfy event types' new schema.
                    // The real value should come from the database record's created_at timestamp,
                    // but it's unclear exactly how.
                    //
                    // See https://github.com/matrix-org/matrix-doc/issues/2064
                    origin_server_ts: 0,
                    prev_content: None,
                    room_id: self.room_id,
                    sender: self.sender,
                    state_key: "".to_string(),
                    unsigned: None,
                })
            }
        }
    };
}

macro_rules! impl_try_into_stripped_state_event_for_event {
    ($ty:ident) => {
        impl TryInto<$ty> for Event {
            type Error = ApiError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                Ok($ty {
                    content: from_str(&self.content).map_err(ApiError::from)?,
                    state_key: "".to_string(),
                    event_type: EventType::from(self.event_type.as_ref()),
                    sender: self.sender,
                })
            }
        }
    };
}

impl_try_into_room_event_for_event!(AnswerEvent);
impl_try_into_room_event_for_event!(CandidatesEvent);
impl_try_into_room_event_for_event!(HangupEvent);
impl_try_into_room_event_for_event!(InviteEvent);
impl_try_into_room_event_for_event!(MessageEvent);
impl_try_into_room_event_for_event!(CustomRoomEvent);

impl_try_into_state_event_for_event!(AliasesEvent);
impl_try_into_state_event_for_event!(AvatarEvent);
impl_try_into_state_event_for_event!(CanonicalAliasEvent);
impl_try_into_state_event_for_event!(CreateEvent);
impl_try_into_state_event_for_event!(GuestAccessEvent);
impl_try_into_state_event_for_event!(HistoryVisibilityEvent);
impl_try_into_state_event_for_event!(JoinRulesEvent);
impl_try_into_state_event_for_event!(NameEvent);
impl_try_into_state_event_for_event!(PowerLevelsEvent);
impl_try_into_state_event_for_event!(ThirdPartyInviteEvent);
impl_try_into_state_event_for_event!(TopicEvent);
impl_try_into_state_event_for_event!(CustomStateEvent);

impl_try_into_stripped_state_event_for_event!(StrippedRoomAliases);
impl_try_into_stripped_state_event_for_event!(StrippedRoomAvatar);
impl_try_into_stripped_state_event_for_event!(StrippedRoomCanonicalAlias);
impl_try_into_stripped_state_event_for_event!(StrippedRoomCreate);
impl_try_into_stripped_state_event_for_event!(StrippedRoomGuestAccess);
impl_try_into_stripped_state_event_for_event!(StrippedRoomHistoryVisibility);
impl_try_into_stripped_state_event_for_event!(StrippedRoomJoinRules);
impl_try_into_stripped_state_event_for_event!(StrippedRoomMember);
impl_try_into_stripped_state_event_for_event!(StrippedRoomName);
impl_try_into_stripped_state_event_for_event!(StrippedRoomPowerLevels);
impl_try_into_stripped_state_event_for_event!(StrippedRoomThirdPartyInvite);
impl_try_into_stripped_state_event_for_event!(StrippedRoomTopic);

impl_try_from_room_event_for_new_event!(AnswerEvent);
impl_try_from_room_event_for_new_event!(CandidatesEvent);
impl_try_from_room_event_for_new_event!(HangupEvent);
impl_try_from_room_event_for_new_event!(InviteEvent);
impl_try_from_room_event_for_new_event!(MessageEvent);
impl_try_from_room_event_for_new_event!(CustomRoomEvent);

impl_try_from_state_event_for_new_event!(AliasesEvent);
impl_try_from_state_event_for_new_event!(AvatarEvent);
impl_try_from_state_event_for_new_event!(CanonicalAliasEvent);
impl_try_from_state_event_for_new_event!(CreateEvent);
impl_try_from_state_event_for_new_event!(GuestAccessEvent);
impl_try_from_state_event_for_new_event!(HistoryVisibilityEvent);
impl_try_from_state_event_for_new_event!(JoinRulesEvent);
impl_try_from_state_event_for_new_event!(MemberEvent);
impl_try_from_state_event_for_new_event!(NameEvent);
impl_try_from_state_event_for_new_event!(PowerLevelsEvent);
impl_try_from_state_event_for_new_event!(ThirdPartyInviteEvent);
impl_try_from_state_event_for_new_event!(TopicEvent);
impl_try_from_state_event_for_new_event!(CustomStateEvent);

impl TryInto<MemberEvent> for Event {
    type Error = ApiError;

    fn try_into(self) -> Result<MemberEvent, Self::Error> {
        Ok(MemberEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            event_type: EventType::RoomMember,
            // FIXME: As of the client-server spec r0.4.0, this data is under the `unsigned` field.
            // Once ruma-events is updated to account for this, this whole TryInto impl can be
            // killed. This is just a dummy value for now to satisfy the old schema.
            invite_room_state: None,
            // FIXME: This is a dummy value just to satisfy event types' new schema.
            // The real value should come from the database record's created_at timestamp,
            // but it's unclear exactly how.
            //
            // See https://github.com/matrix-org/matrix-doc/issues/2064
            origin_server_ts: 0,
            prev_content: None,
            room_id: self.room_id,
            sender: self.sender,
            state_key: "".to_string(),
            unsigned: None,
        })
    }
}

impl TryInto<StateEvent> for Event {
    type Error = ApiError;

    fn try_into(self) -> Result<StateEvent, Self::Error> {
        let state_event = match EventType::from(self.event_type.as_ref()) {
            EventType::RoomAliases => StateEvent::RoomAliases(self.try_into()?),
            EventType::RoomAvatar => StateEvent::RoomAvatar(self.try_into()?),
            EventType::RoomCanonicalAlias => StateEvent::RoomCanonicalAlias(self.try_into()?),
            EventType::RoomCreate => StateEvent::RoomCreate(self.try_into()?),
            EventType::RoomGuestAccess => StateEvent::RoomGuestAccess(self.try_into()?),
            EventType::RoomHistoryVisibility => StateEvent::RoomHistoryVisibility(self.try_into()?),
            EventType::RoomJoinRules => StateEvent::RoomJoinRules(self.try_into()?),
            EventType::RoomMember => StateEvent::RoomMember(self.try_into()?),
            EventType::RoomName => StateEvent::RoomName(self.try_into()?),
            EventType::RoomPowerLevels => StateEvent::RoomPowerLevels(self.try_into()?),
            EventType::RoomThirdPartyInvite => StateEvent::RoomThirdPartyInvite(self.try_into()?),
            EventType::RoomTopic => StateEvent::RoomTopic(self.try_into()?),
            _ => Err(ApiError::bad_event(format!("Unknown state event type {}", self.event_type)))?,
        };

        Ok(state_event)
    }
}

impl TryInto<StrippedState> for Event {
    type Error = ApiError;

    fn try_into(self) -> Result<StrippedState, Self::Error> {
        let stripped_state_event = match EventType::from(self.event_type.as_ref()) {
            EventType::RoomAliases => StrippedState::RoomAliases(self.try_into()?),
            EventType::RoomAvatar => StrippedState::RoomAvatar(self.try_into()?),
            EventType::RoomCanonicalAlias => StrippedState::RoomCanonicalAlias(self.try_into()?),
            EventType::RoomCreate => StrippedState::RoomCreate(self.try_into()?),
            EventType::RoomGuestAccess => StrippedState::RoomGuestAccess(self.try_into()?),
            EventType::RoomHistoryVisibility => StrippedState::RoomHistoryVisibility(self.try_into()?),
            EventType::RoomJoinRules => StrippedState::RoomJoinRules(self.try_into()?),
            EventType::RoomMember => StrippedState::RoomMember(self.try_into()?),
            EventType::RoomName => StrippedState::RoomName(self.try_into()?),
            EventType::RoomPowerLevels => StrippedState::RoomPowerLevels(self.try_into()?),
            EventType::RoomThirdPartyInvite => StrippedState::RoomThirdPartyInvite(self.try_into()?),
            EventType::RoomTopic => StrippedState::RoomTopic(self.try_into()?),
            _ => Err(ApiError::bad_event(format!("Unknown state event type {}", self.event_type)))?,
        };

        Ok(stripped_state_event)
    }
}
