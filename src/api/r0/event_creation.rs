//! Endpoints for creating events.

use std::convert::{TryFrom, TryInto};

use bodyparser;
use diesel::{Connection, ExecuteDsl, FindDsl, LoadDsl, insert};
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use router::Router;
use ruma_identifiers::{EventId, RoomId, UserId};
use ruma_events::{CustomRoomEvent, EventType};
use ruma_events::call::answer::AnswerEvent;
use ruma_events::call::candidates::CandidatesEvent;
use ruma_events::call::hangup::HangupEvent;
use ruma_events::call::invite::InviteEvent;
use ruma_events::room::message::MessageEvent;
use serde_json::from_value;

use db::DB;
use config::Config;
use error::APIError;
use event::NewEvent;
use middleware::{AccessTokenAuth, JsonRequest};
use modifier::SerializableResponse;
use room::Room;
use schema::{events, rooms};
use user::User;

#[derive(Debug, Serialize)]
struct SendMessageEventResponse {
    event_id: String,
}

/// The /rooms/:room_id/send/:event_type/:transaction_id endpoint.
pub struct SendMessageEvent;

impl SendMessageEvent {
    /// Create a `SendMessageEvent` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(SendMessageEvent);

        chain.link_before(JsonRequest);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for SendMessageEvent {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_id = match params.find("room_id") {
            Some(room_id) => RoomId::try_from(room_id).map_err(APIError::from)?,
            None => {
                let error = APIError::missing_param("room_id");

                return Err(IronError::new(error.clone(), error));
            }
        };

        let event_type = params
            .find("event_type")
            .ok_or(APIError::missing_param("event_type"))
            .map(EventType::from)?;

        let transaction_id = params
            .find("transaction_id")
            .ok_or(APIError::missing_param("transaction_id"));

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let event_content = request
            .get::<bodyparser::Json>()
            .expect("JsonRequest verifies the Result is Ok")
            .expect("JsonRequest verifies the Option is Some");
        let config = Config::from_request(request)?;
        let event_id = EventId::new(&config.domain).map_err(APIError::from)?;
        let user_id = UserId::try_from(&format!("@{}:{}", user.id, config.domain))
            .map_err(APIError::from)?;

        let room_event: NewEvent = match event_type {
            EventType::CallAnswer => {
                AnswerEvent {
                    content: from_value(event_content).map_err(APIError::from)?,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: event_type,
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            EventType::CallCandidates => {
                CandidatesEvent {
                    content: from_value(event_content).map_err(APIError::from)?,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: event_type,
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            EventType::CallHangup => {
                HangupEvent {
                    content: from_value(event_content).map_err(APIError::from)?,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: event_type,
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            EventType::CallInvite => {
                InviteEvent {
                    content: from_value(event_content).map_err(APIError::from)?,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: event_type,
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            EventType::RoomMessage => {
                MessageEvent {
                    content: from_value(event_content).map_err(APIError::from)?,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: event_type,
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            EventType::Custom(custom_event_type) => {
                CustomRoomEvent {
                    content: event_content,
                    event_id: event_id.clone(),
                    extra_content: (),
                    event_type: EventType::Custom(custom_event_type),
                    room_id: room_id.clone(),
                    unsigned: None,
                    user_id: user_id,
                }.try_into().map_err(APIError::from)?
            }
            _ => {
                let error = APIError::bad_event();

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;

        connection.transaction(|| {
            if let Err(_) = rooms::table.find(room_id.to_string()).first::<Room>(&*connection) {
                return Err(APIError::not_found());
            };

            insert(&room_event)
                .into(events::table)
                .execute(&*connection)
                .map_err(APIError::from)
        }).map_err(APIError::from)?;

        let response = SendMessageEventResponse {
            event_id: event_id.opaque_id().to_string(),
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}
