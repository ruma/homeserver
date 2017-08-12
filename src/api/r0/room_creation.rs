//! Endpoints for room creation.

use std::convert::From;

use bodyparser;
use diesel::Connection;
use iron::{Chain, Handler, IronResult, Plugin, Request, Response};
use iron::status::Status;
use ruma_events::stripped::StrippedState;
use ruma_identifiers::{RoomId, UserId};

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain};
use models::room::{CreationOptions, NewRoom, Room, RoomPreset, RoomVisibility};
use models::room_membership::{RoomMembership, RoomMembershipOptions};
use models::user::User;
use modifier::SerializableResponse;

/// The `/createRoom` endpoint.
pub struct CreateRoom;

#[derive(Clone, Debug, Deserialize)]
struct CreateRoomRequest {
    /// Extra keys to be added to the content of the m.room.create.
    pub creation_content: Option<CreationContent>,
    /// A list of state events to set in the new room. This allows the
    /// user to override the default state events set in the new room.
    pub initial_state: Option<Vec<Box<StrippedState>>>,
    /// A list of user IDs to invite to the room.
    pub invite: Option<Vec<UserId>>,
    /// Indicates the room's name.
    pub name: Option<String>,
    /// Convenience parameter for setting various default state events based on a preset.
    pub preset: Option<RoomPreset>,
    /// The desired room alias local part.
    pub room_alias_name: Option<String>,
    /// Indicates the room's topic.
    pub topic: Option<String>,
    /// Indicates whether or not that the room will be shown in the published room list.
    pub visibility: Option<RoomVisibility>,
}

#[derive(Clone, Debug, Deserialize)]
struct CreationContent {
    #[serde(rename="m.federate")]
    pub federate: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CreateRoomResponse {
    /// The fully qualified ID of the room that was created.
    room_id: RoomId,
}

middleware_chain!(CreateRoom, [JsonRequest, AccessTokenAuth]);

impl Handler for CreateRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();
        let create_room_request = match request.get::<bodyparser::Struct<CreateRoomRequest>>() {
            Ok(Some(create_room_request)) => create_room_request,
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let new_room = NewRoom {
            id: RoomId::new(&config.domain).map_err(ApiError::from)?,
            user_id: user.id,
            public: create_room_request.visibility.map_or(false, |v| v == RoomVisibility::Public),
        };

        let federate = match create_room_request.creation_content {
            Some(creation_content) => creation_content.federate.unwrap_or(true),
            None => true,
        };

        let preset = match create_room_request.preset {
            Some(preset) => preset,
            None => if new_room.public {
                RoomPreset::PublicChat
            } else {
                RoomPreset::PrivateChat
            }
        };

        let creation_options = CreationOptions {
            alias: create_room_request.room_alias_name,
            federate: Some(federate),
            initial_state: create_room_request.initial_state,
            invite_list: create_room_request.invite,
            name: create_room_request.name,
            preset: preset,
            topic: create_room_request.topic,
        };

        let room: Room = connection.transaction::<Room, ApiError, _>(|| {
            let room = Room::create(&connection, &new_room, &config.domain, &creation_options)?;

            let options = RoomMembershipOptions {
                room_id: room.id.clone(),
                user_id: room.user_id.clone(),
                sender: room.user_id.clone(),
                membership: "join".to_string(),
            };

            RoomMembership::create(&connection, &config.domain, options)?;

            Ok(room)
        })
        .map_err(ApiError::from)?;

        let response = CreateRoomResponse {
            room_id: room.id,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn no_parameters() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);

        let response = test.post(&create_room_path, "{}");

        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_room_alias() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);

        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().get("room_id").unwrap().as_str();

        assert!(room_id.is_some());

        let alias_response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(
            alias_response.json().get("room_id").unwrap().as_str().unwrap(),
            room_id.unwrap()
        );
    }

    #[test]
    fn with_public_visibility() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);

        let response = test.post(&create_room_path, r#"{"visibility": "public"}"#);

        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_private_visibility() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);

        let response = test.post(&create_room_path, r#"{"visibility": "private"}"#);

        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_invalid_visibility() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);

        let response = test.post(&create_room_path, r#"{"visibility": "bogus"}"#);

        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "M_BAD_JSON"
        );
    }

    #[test]
    fn with_invited_users() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();
        let carl = test.create_user();

        let room_options = format!(r#"{{"visibility": "private",
                                        "invite": [
                                           "{}",
                                           "{}"
                                        ]}}"#, bob.id, carl.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert!(test.join_room(&alice.token, &room_id).status.is_success());
        assert!(test.join_room(&bob.token, &room_id).status.is_success());
        assert!(test.join_room(&carl.token, &room_id).status.is_success());
    }

    #[test]
    fn with_unknown_invited_users() {
        let test = Test::new();
        let _bob = test.create_user();
        let alice = test.create_user();

        let room_options = r#"{"visibility": "private",
                               "invite": [
                                   "@bob:ruma.test",
                                   "@carl:ruma.test",
                                   "@dan:ruma.test"
                               ]}"#;

        let response = test.post(
            &format!( "/_matrix/client/r0/createRoom?access_token={}", alice.token),
            room_options
        );

        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "M_BAD_JSON"
        );

        let error = response.json().get("error").unwrap().as_str().unwrap().to_string();

        assert!(error.starts_with("Unknown users in invite list:"));
        assert!(error.contains("@carl:ruma.test"));
        assert!(error.contains("@dan:ruma.test"));
    }

    #[test]
    fn creator_has_max_power_level_from_initial_state() {
        let test = Test::new();

        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{
            "invite": [ "{}" ],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.power_levels",
                "content": {{
                    "ban": 100,
                    "events": {{ "m.room.message": 100 }},
                    "events_default": 0,
                    "invite": 100,
                    "kick": 100,
                    "redact": 0,
                    "state_default": 0,
                    "users": {{  }},
                    "users_default": 0
                }}
            }}]
        }}"#, bob.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&alice.token, &room_id, "Hi", 1);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&bob.token, &room_id, "Hi", 2);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "Insufficient power level to create this event."
        );
    }

    #[test]
    fn creator_has_max_power_level_by_default() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures("{}");
        let bob = test.create_user();
        let carl = test.create_user();

        let response = test.invite(&alice.token, &room_id, &bob.id);
        assert_eq!(response.status, Status::Ok);

        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let response = test.invite(&bob.token, &room_id, &carl.id);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "Insufficient power level to invite a user"
        );
    }

    #[test]
    fn with_power_levels_in_initial_state() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();
        let carl = test.create_user();
        let dan = test.create_user();
        let eve = test.create_user();

        let room_options = format!(r#"{{
            "invite": [
                "{0}",
                "{1}"
            ],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.power_levels",
                "content": {{
                    "ban": 100,
                    "events": {{ "m.room.topic": 50 }},
                    "events_default": 0,
                    "invite": 100,
                    "kick": 100,
                    "redact": 0,
                    "state_default": 0,
                    "users": {{
                        "{0}": 100,
                        "{1}": 50
                    }},
                    "users_default": 0
                }}
            }}]
        }}"#, bob.id, carl.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);
        assert_eq!(test.join_room(&carl.token, &room_id).status, Status::Ok);

        // Bob has enough power to invite other users.
        assert_eq!(
            test.invite(&bob.token, &room_id, &eve.id).status,
            Status::Ok
        );

        // Carl doesn't ...
        assert_eq!(
            test.invite(&carl.token, &room_id, &dan.id).status,
            Status::Forbidden
        );
    }

    #[test]
    fn with_room_aliases_in_initial_state() {
        let test = Test::new();
        let alice = test.create_user();

        let room_options = r##"{
            "initial_state": [{
                "state_key": "",
                "type": "m.room.aliases",
                "content": {
                    "aliases": ["#alias_1:ruma.test", "#alias_2:ruma.test"]
                }
            }]
        }"##;

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let first_alias_response = test.get_room_by_alias("alias_1");
        let second_alias_response = test.get_room_by_alias("alias_2");

        assert_eq!(
            first_alias_response.json().get("room_id").unwrap().as_str().unwrap(),
            room_id
        );

        assert_eq!(
            second_alias_response.json().get("room_id").unwrap().as_str().unwrap(),
            room_id
        );
    }

    #[test]
    fn with_join_rules_in_initial_state() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = r#"{
            "initial_state":[{
                "state_key": "",
                "content": { "join_rule": "public" },
                "type": "m.room.join_rules"
            }]
        }"#;

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        // Bob can join without an invite.
        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);
    }

    #[test]
    fn with_increased_power_levels_in_trusted_chats_by_default() {
        let test = Test::new();

        let carl = test.create_user();
        let bob = test.create_user();
        let alice = test.create_user();

        let room_options = format!(r#"{{
            "invite": ["{}", "{}"],
            "preset": "trusted_private_chat"
        }}"#, bob.id, carl.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);
        assert_eq!(test.invite(&bob.token, &room_id, &carl.id).status, Status::Ok);
    }

    #[test]
    fn with_increased_power_levels_in_trusted_chats_from_initial_state() {
        let test = Test::new();

        let carl = test.create_user();
        let bob = test.create_user();
        let alice = test.create_user();

        let room_options = format!(r#"{{
            "invite": ["{}"],
            "preset": "trusted_private_chat",
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.power_levels",
                "content": {{
                    "ban": 100,
                    "events": {{ "m.message.text": 100 }},
                    "events_default": 100,
                    "invite": 100,
                    "kick": 100,
                    "redact": 100,
                    "state_default": 100,
                    "users": {{  }},
                    "users_default": 0
                }}
            }}]
        }}"#, bob.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);
        assert_eq!(test.invite(&bob.token, &room_id, &carl.id).status, Status::Ok);
        assert_eq!(test.send_message(&bob.token, &room_id, "Hi", 1).status, Status::Ok);
        assert_eq!(test.send_message(&alice.token, &room_id, "Hi", 1).status, Status::Ok);
    }
}
