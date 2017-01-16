//! Matrix events.

use std::convert::{TryInto, TryFrom};

use diesel::{
    ExpressionMethods,
    FilterDsl,
    FindDsl,
    LoadDsl,
    OrderDsl,
    TextExpressionMethods,
};
use diesel::types::{BigSerial, Nullable, Text, Timestamp};
use diesel::expression::dsl::sql;
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
use ruma_events::collections::all::StateEvent;
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
#[derive(Debug, Clone, Insertable)]
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

    /// Return `RoomEvent`'s by `RoomId` and since.
    pub fn find_room_events(connection: &PgConnection, room_id: &RoomId, since: i64) -> Result<Vec<Event>, ApiError> {
        let events: Vec<Event> = events::table
            .filter(events::event_type.like("m.room.%"))
            .filter(events::ordering.gt(since))
            .filter(events::room_id.eq(room_id))
            .order(events::room_id)
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;

        Ok(events)
    }

    /// Look up an event given its `EventId`.
    pub fn find(connection: &PgConnection, event_id: &EventId) -> Result<Option<Event>, ApiError> {
        match events::table.find(event_id).first(connection) {
            Ok(event) => Ok(Some(event)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Return the latest state events for a room.
    ///
    /// A pivot event is used to specify the latest moment in time that it will be
    /// used to extract the state snapshot. If the pivot event is not present,
    /// the latest state events are extracted.
    pub fn get_room_state_events_until(connection: &PgConnection, room_id: &RoomId, pivot_event: Option<Event>)
    -> Result<Vec<Event>, ApiError> {
        let ordering_filter = match pivot_event {
            Some(event) => format!("AND events.ordering < {}", event.ordering),
            None => "".to_string()
        };

        let grouped_sql = format!("
            SELECT events.event_type, MAX(events.ordering) AS max_ordering
            FROM events
            WHERE events.room_id = '{}'
            {}
            AND events.event_type IN (
                'm.room.aliases',
                'm.room.avatar',
                'm.room.canonical_alias',
                'm.room.create',
                'm.room.guest_access',
                'm.room.history_visibility',
                'm.room.join_rules',
                'm.room.name',
                'm.room.power_levels',
                'm.room.third_party_invite',
                'm.room.topic'
            )
            GROUP BY events.event_type
            ORDER BY MAX(events.ordering)
        ", room_id.to_string(), ordering_filter);

        let query = format!(r#"
            SELECT  all_events.id,
                    all_events.ordering,
                    all_events.room_id,
                    all_events.user_id,
                    all_events.event_type,
                    all_events.state_key,
                    all_events.content,
                    all_events.extra_content,
                    all_events.created_at
            FROM ( {} ) AS grouped
            INNER JOIN events AS all_events
            ON grouped.event_type = all_events.event_type
            AND grouped.max_ordering = all_events.ordering;
        "#, grouped_sql);

        let events = sql::<(Text,
                            BigSerial,
                            Text,
                            Text,
                            Text,
                            Nullable<Text>,
                            Text,
                            Nullable<Text>,
                            Timestamp)>(&query)
            .get_results::<Event>(connection)?;

        Ok(events)
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

macro_rules! impl_try_into_room_event_for_event {
    ($ty:ident) => {
        impl TryInto<$ty> for Event {
            type Err = ApiError;

            fn try_into(self) -> Result<$ty, Self::Err> {
                Ok($ty {
                    content: from_str(&self.content).map_err(ApiError::from)?,
                    event_id: self.id,
                    event_type: EventType::from(self.event_type.as_ref()),
                    room_id: self.room_id,
                    unsigned: None,
                    user_id: self.user_id,
                })
            }
        }
    };
}
macro_rules! impl_try_into_state_event_for_event {
    ($ty:ident) => {
        impl TryInto<$ty> for Event {
            type Err = ApiError;

            fn try_into(self) -> Result<$ty, Self::Err> {
                Ok($ty {
                    content: from_str(&self.content).map_err(ApiError::from)?,
                    prev_content: None,
                    event_id: self.id,
                    state_key: "".to_string(),
                    event_type: EventType::from(self.event_type.as_ref()),
                    room_id: self.room_id,
                    unsigned: None,
                    user_id: self.user_id,
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
    type Err = ApiError;

    fn try_into(self) -> Result<MemberEvent, Self::Err> {
        Ok(MemberEvent {
            content: from_str(&self.content)?,
            event_id: self.id,
            invite_room_state: match self.extra_content {
                Some(extra_content) => {
                    let object: Value = from_str(&extra_content).map_err(ApiError::from)?;
                    let field: &Value = object.find("invite_room_state")
                        .ok_or_else(||
                            ApiError::unknown("Data for member event was missing invite_room_state".to_string()
                    ))?;


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

impl TryInto<StateEvent> for Event {
    type Err = ApiError;

    fn try_into(self) -> Result<StateEvent, Self::Err> {
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
