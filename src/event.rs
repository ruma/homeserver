//! Matrix events.

use std::convert::{TryInto, TryFrom};

use diesel::{ExpressionMethods, FilterDsl, LoadDsl};
use diesel::result::Error as DieselError;
use diesel::pg::data_types::PgTimestamp;
use diesel::pg::PgConnection;
use ruma_events::{
    CustomRoomEvent,
    CustomStateEvent,
    Event as RumaEventsEvent,
    EventType,
    RoomEvent,
    StateEvent,
};
use ruma_events::call::answer::AnswerEvent;
use ruma_events::call::candidates::CandidatesEvent;
use ruma_events::call::hangup::HangupEvent;
use ruma_events::call::invite::InviteEvent;
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
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{Value, from_str, from_value, to_string};

use error::ApiError;
use schema::events;

/// A new event, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "events"]
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

macro_rules! impl_try_from_room_event_for_new_event {
    ($ty:ty) => {
        impl TryFrom<$ty> for NewEvent {
            type Err = ApiError;

            fn try_from(event: $ty) -> Result<Self, Self::Err> {
                Ok(NewEvent {
                    content: to_string(event.content()).map_err(ApiError::from)?,
                    event_type: event.event_type().to_string(),
                    extra_content: None,
                    id: event.event_id().clone(),
                    room_id: event.room_id().clone(),
                    state_key: None,
                    user_id: event.user_id().clone(),
                })
            }
        }
    }
}

macro_rules! impl_try_from_state_event_for_new_event {
    ($ty:ty) => {
        impl TryFrom<$ty> for NewEvent {
            type Err = ApiError;

            fn try_from(event: $ty) -> Result<Self, Self::Err> {
                Ok(NewEvent {
                    content: to_string(event.content()).map_err(ApiError::from)?,
                    event_type: event.event_type().to_string(),
                    extra_content: match event.extra_content() {
                        Some(extra_content) => Some(
                            to_string(&extra_content).map_err(ApiError::from)?
                        ),
                        None => None,
                    },
                    id: event.event_id().clone(),
                    room_id: event.room_id().clone(),
                    state_key: Some(event.state_key().to_string()),
                    user_id: event.user_id().clone(),
                })
            }
        }
    }
}

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

impl TryInto<JoinRulesEvent> for Event {
    type Err = ApiError;

    fn try_into(self) -> Result<JoinRulesEvent, Self::Err> {
        Ok(JoinRulesEvent {
            content: from_str(&self.content).map_err(ApiError::from)?,
            event_id: self.id,
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
    type Err = ApiError;

    fn try_into(self) -> Result<MemberEvent, Self::Err> {
        Ok(MemberEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            invite_room_state: match self.extra_content {
                Some(extra_content) => {
                    let object: Value = from_str(&extra_content).map_err(ApiError::from)?;

                    let field: &Value = object.find("invite_room_state").ok_or(
                        ApiError::unknown(
                            Some("Data for member event was missing invite_room_state")
                        )
                    )?;

                    from_value(field.clone()).map_err(ApiError::from)?
                },
                None => None,
            },
            prev_content: None,
            state_key: "".to_string(),
            event_type: EventType::RoomMember,
            room_id: self.room_id,
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

impl TryInto<PowerLevelsEvent> for Event {
    type Err = ApiError;

    fn try_into(self) -> Result<PowerLevelsEvent, Self::Err> {
        Ok(PowerLevelsEvent {
            content: from_str(&self.content).map_err(ApiError::from)?,
            event_id: self.id,
            event_type: EventType::RoomPowerLevels,
            prev_content: None,
            room_id: self.room_id,
            state_key: "".to_string(),
            unsigned: None,
            user_id: self.user_id,
        })
    }
}

