//! Endpoints for creating events.

use std::convert::TryInto;

use bodyparser;
use diesel::{Connection, ExecuteDsl, FindDsl, LoadDsl, insert};
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use router::Router;
use ruma_events::call::answer::AnswerEvent;
use ruma_events::call::candidates::CandidatesEvent;
use ruma_events::call::hangup::HangupEvent;
use ruma_events::call::invite::InviteEvent;
use ruma_events::room::avatar::AvatarEvent;
use ruma_events::room::canonical_alias::CanonicalAliasEvent;
use ruma_events::room::guest_access::GuestAccessEvent;
use ruma_events::room::history_visibility::HistoryVisibilityEvent;
use ruma_events::room::join_rules::JoinRulesEvent;
use ruma_events::room::message::MessageEvent;
use ruma_events::room::name::NameEvent;
use ruma_events::room::power_levels::PowerLevelsEvent;
use ruma_events::room::third_party_invite::ThirdPartyInviteEvent;
use ruma_events::room::topic::TopicEvent;
use ruma_events::{CustomRoomEvent, CustomStateEvent, EventType};
use ruma_identifiers::EventId;
use serde::Deserialize;
use serde_json::{Value, from_value};

use db::DB;
use config::Config;
use error::{ApiError, MapApiError};
use event::NewEvent;
use middleware::{AccessTokenAuth, JsonRequest, RoomIdParam};
use modifier::SerializableResponse;
use room::Room;
use schema::{events, rooms};
use user::User;

macro_rules! room_event {
    (
        $ty:ident,
        $event_content:ident,
        $event_type:ident,
        $event_id:ident,
        $room_id:ident,
        $user:ident
    ) => {
        $ty {
            content: extract_event_content($event_content, &$event_type)?,
            event_id: $event_id.clone(),
            extra_content: (),
            event_type: $event_type.clone(),
            room_id: $room_id.clone(),
            unsigned: None,
            user_id: $user.id.clone(),
        }.try_into().map_err(ApiError::from)?
    };
}

macro_rules! state_event {
    (
        $ty:ident,
        $event_content:ident,
        $event_type:ident,
        $event_id:ident,
        $room_id:ident,
        $state_key:ident,
        $user:ident
    ) => {
        $ty {
            content: extract_event_content($event_content, &$event_type)?,
            event_id: $event_id.clone(),
            extra_content: (),
            event_type: $event_type.clone(),
            prev_content: None,
            room_id: $room_id.clone(),
            state_key: $state_key.to_string(),
            unsigned: None,
            user_id: $user.id.clone(),
        }.try_into().map_err(ApiError::from)?
    };
}

#[derive(Debug, Serialize)]
struct EventResponse {
    event_id: String,
}

/// The /rooms/:room_id/send/:event_type/:transaction_id endpoint.
pub struct SendMessageEvent;

/// The /rooms/:room_id/state/:event_type/:state_key and /rooms/:room_id/state/:event_type
/// endpoints.
pub struct StateMessageEvent;

impl SendMessageEvent {
    /// Create a `SendMessageEvent` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(SendMessageEvent);

        chain.link_before(JsonRequest);
        chain.link_before(RoomIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl StateMessageEvent {
    /// Create a `StateMessageEvent` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(StateMessageEvent);

        chain.link_before(JsonRequest);
        chain.link_before(RoomIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for SendMessageEvent {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("Should have been required by RoomIdParam.")
            .clone();

        let event_type = params
            .find("event_type")
            .ok_or(ApiError::missing_param("event_type"))
            .map(EventType::from)?;

        let transaction_id = params
            .find("transaction_id")
            .ok_or(ApiError::missing_param("transaction_id"));

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let event_content = request
            .get::<bodyparser::Json>()
            .expect("JsonRequest verifies the Result is Ok")
            .expect("JsonRequest verifies the Option is Some");
        let config = Config::from_request(request)?;
        let event_id = EventId::new(&config.domain).map_api_err(|_| {
            ApiError::unknown(Some("Failed to generated event ID for the new event."))
        })?;

        let room_event: NewEvent = match event_type {
            EventType::CallAnswer => {
                room_event!(AnswerEvent, event_content, event_type, event_id, room_id, user)
            }
            EventType::CallCandidates => {
                room_event!(CandidatesEvent, event_content, event_type, event_id, room_id, user)
            }
            EventType::CallHangup => {
                room_event!(HangupEvent, event_content, event_type, event_id, room_id, user)
            }
            EventType::CallInvite => {
                room_event!(InviteEvent, event_content, event_type, event_id, room_id, user)
            }
            EventType::RoomMessage => {
                room_event!(MessageEvent, event_content, event_type, event_id, room_id, user)
            }
            EventType::Custom(ref custom_event_type) => {
                CustomRoomEvent {
                    content: event_content,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: EventType::Custom(custom_event_type.clone()),
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user.id.clone(),
                }.try_into().map_err(ApiError::from)?
            }
            _ => {
                let error = ApiError::bad_event(
                    Some(&format!("Events of type {} cannot be created with this API.", event_type))
                );

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;

        connection.transaction(|| {
            let room = rooms::table.find(room_id.to_string()).first::<Room>(&*connection)?;
            let power_levels = room.current_power_levels(&*connection)?;
            let user_power_level = power_levels
                .users
                .get(&user.id)
                .unwrap_or(&power_levels.users_default);
            let required_power_level = power_levels
                .events
                .get(&event_type)
                .unwrap_or(&power_levels.events_default);

            if required_power_level > user_power_level {
                return Err(
                    ApiError::unauthorized(Some("Insufficient power level to create this event."))
                );
            }

            insert(&room_event)
                .into(events::table)
                .execute(&*connection)
                .map_err(ApiError::from)
        }).map_err(ApiError::from)?;

        let response = EventResponse {
            event_id: event_id.opaque_id().to_string(),
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}

impl Handler for StateMessageEvent {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("Should have been required by RoomIdParam.")
            .clone();

        let event_type = params
            .find("event_type")
            .ok_or(ApiError::missing_param("event_type"))
            .map(EventType::from)?;

        let state_key = params
            .find("state_key")
            .unwrap_or("");

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let event_content = request
            .get::<bodyparser::Json>()
            .expect("JsonRequest verifies the Result is Ok")
            .expect("JsonRequest verifies the Option is Some");
        let config = Config::from_request(request)?;
        let event_id = EventId::new(&config.domain).map_api_err(|_| {
            ApiError::unknown(Some("Failed to generated event ID for the new event."))
        })?;

        let state_event: NewEvent = match event_type {
            EventType::RoomAvatar => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    AvatarEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomCanonicalAlias => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    CanonicalAliasEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomGuestAccess => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    GuestAccessEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomHistoryVisibility => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    HistoryVisibilityEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomJoinRules => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    JoinRulesEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomName => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    NameEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomPowerLevels => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    PowerLevelsEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomThirdPartyInvite => {
                state_event!(
                    ThirdPartyInviteEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::RoomTopic => {
                ensure_empty_state_key(state_key)?;

                state_event!(
                    TopicEvent,
                    event_content,
                    event_type,
                    event_id,
                    room_id,
                    state_key,
                    user
                )
            }
            EventType::Custom(ref custom_event_type) => {
                CustomStateEvent {
                    content: event_content,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: EventType::Custom(custom_event_type.clone()),
                    prev_content: None,
                    room_id: room_id.clone(),
                    state_key: state_key.to_string(),
                    unsigned: None,
                    user_id: user.id.clone(),
                }.try_into().map_err(ApiError::from)?
            }
            _ => {
                let error = ApiError::bad_event(
                    Some(&format!("Events of type {} cannot be created with this API.", event_type))
                );

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;

        connection.transaction(|| {
            let room = rooms::table.find(room_id.to_string()).first::<Room>(&*connection)?;
            let power_levels = room.current_power_levels(&*connection)?;
            let user_power_level = power_levels
                .users
                .get(&user.id)
                .unwrap_or(&power_levels.users_default);
            let required_power_level = power_levels
                .events
                .get(&event_type)
                .unwrap_or(&power_levels.state_default);

            if required_power_level > user_power_level {
                return Err(
                    ApiError::unauthorized(Some("Insufficient power level to create this event."))
                );
            }

            insert(&state_event)
                .into(events::table)
                .execute(&*connection)
                .map_err(ApiError::from)
        }).map_err(ApiError::from)?;

        let response = EventResponse {
            event_id: event_id.opaque_id().to_string(),
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}

/// Enforces an empty state key for an event type that requires it.
fn ensure_empty_state_key(state_key: &str) -> Result<(), IronError> {
    if state_key == "" {
        Ok(())
    } else {
        let error = ApiError::bad_event(Some("Events of type {} must have an empty state key."));

        Err(IronError::new(error.clone(), error))
    }
}

/// Convert the JSON from the request into the correct type for the event's `content` field.
fn extract_event_content<T: Deserialize>(event_content: Value, event_type: &EventType)
-> Result<T, ApiError> {
    from_value(event_content).map_api_err(|_| {
        ApiError::bad_event(
            Some(
                &format!(
                    "Event content did not match expected structure for event of type {}.",
                    event_type
                )
            )
        )
    })
}

#[cfg(test)]
mod tests {
    use test::Test;

    #[test]
    fn create_message_event() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room(&access_token);

        let create_event_path = format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/1?access_token={}",
            room_id,
            access_token
        );

        let response = test.put(&create_event_path, r#"{"body":"Hi","msgtype":"m.text"}"#);

        assert!(response.json().find("event_id").unwrap().as_str().is_some());
    }

    #[test]
    fn event_content_does_not_match_event_type() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room(&access_token);

        let create_event_path = format!(
            "/_matrix/client/r0/rooms/{}/send/m.call.answer/1?access_token={}",
            room_id,
            access_token
        );

        let response = test.put(&create_event_path, r#"{"body":"Hi","msgtype":"m.text"}"#);

        let json = response.json();

        assert_eq!(json.find("errcode").unwrap().as_str().unwrap(), "IO_RUMA_BAD_EVENT");
        assert_eq!(
            json.find("error").unwrap().as_str().unwrap(),
            "Event content did not match expected structure for event of type m.call.answer."
        );
    }

    #[test]
    fn non_message_event_type() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room(&access_token);

        let create_event_path = format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.topic/1?access_token={}",
            room_id,
            access_token
        );

        let response = test.put(&create_event_path, r#"{"topic":"fail"}"#);

        let json = response.json();

        assert_eq!(json.find("errcode").unwrap().as_str().unwrap(), "IO_RUMA_BAD_EVENT");
        assert_eq!(
            json.find("error").unwrap().as_str().unwrap(),
            "Events of type m.room.topic cannot be created with this API."
        );
    }

    #[test]
    fn custom_message_event() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room(&access_token);

        let create_event_path = format!(
            "/_matrix/client/r0/rooms/{}/send/io.ruma.test/1?access_token={}",
            room_id,
            access_token
        );

        let response = test.put(&create_event_path, r#"{"foo":"bar"}"#);

        assert!(response.json().find("event_id").unwrap().as_str().is_some());
    }
}
