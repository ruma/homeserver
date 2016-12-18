//! Endpoints for room creation.

use std::convert::From;

use bodyparser;
use diesel::Connection;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;
use ruma_identifiers::RoomId;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain};
use models::room::{CreationOptions, NewRoom, Room, RoomPreset};
use models::room_membership::{RoomMembership, RoomMembershipOptions};
use models::user::User;
use modifier::SerializableResponse;

/// The `/createRoom` endpoint.
pub struct CreateRoom;

#[derive(Clone, Debug, Deserialize)]
struct CreateRoomRequest {
    pub creation_content: Option<CreationContent>,
    pub invite: Option<Vec<String>>,
    pub name: Option<String>,
    pub preset: Option<RoomPreset>,
    pub room_alias_name: Option<String>,
    pub topic: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CreationContent {
    #[serde(rename="m.federate")]
    pub federate: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CreateRoomResponse {
    room_id: String,
}

middleware_chain!(CreateRoom, [JsonRequest, AccessTokenAuth]);

impl CreateRoomRequest {
    pub fn validate(self) -> Result<Self, IronError> {
        if let Some(ref visibility) = self.visibility {
            if visibility != "public" && visibility != "private" {
                return Err(IronError::from(ApiError::bad_json(None)));
            }
        }

        Ok(self)
    }
}

impl Handler for CreateRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();
        let create_room_request = match request.get::<bodyparser::Struct<CreateRoomRequest>>() {
            Ok(Some(create_room_request)) => create_room_request.validate()?,
            Ok(None) | Err(_) => {
                return Err(IronError::from(ApiError::bad_json(None)));
            }
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let new_room = NewRoom {
            id: RoomId::new(&config.domain).map_err(ApiError::from)?,
            user_id: user.id,
            public: create_room_request.visibility.map_or(false, |v| v == "public"),
        };

        let federate = match create_room_request.creation_content {
            Some(creation_content) => creation_content.federate.unwrap_or(true),
            None => true,
        };

        let preset = match create_room_request.preset {
            Some(preset) => preset,
            None => match new_room.public {
                true => RoomPreset::PublicChat,
                false => RoomPreset::PrivateChat,
            }
        };

        let creation_options = CreationOptions {
            alias: create_room_request.room_alias_name,
            federate: federate,
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

            RoomMembership::create(&connection, &config.domain, options)
                .map_err(ApiError::from)?;

            Ok(room)
        })
        .map_err(ApiError::from)?;

        let response = CreateRoomResponse {
            room_id: room.id.to_string(),
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;

    #[test]
    fn no_parameters() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);

        let response = test.post(&create_room_path, "{}");

        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);

        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().find("room_id").unwrap().as_str();

        assert!(room_id.is_some());

        let alias_response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(
            alias_response.json().find("room_id").unwrap().as_str().unwrap(),
            room_id.unwrap()
        );
    }

    #[test]
    fn with_public_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "public"}"#);

        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_private_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "private"}"#);

        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn with_invalid_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "bogus"}"#);

        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_BAD_JSON"
        );
    }

    #[test]
    fn with_invited_users() {
        let test = Test::new();
        let alice_token = test.create_access_token_with_username("alice");
        let bob_token = test.create_access_token_with_username("bob");
        let carl_token = test.create_access_token_with_username("carl");

        let room_options = r#"{"visibility": "private",
                               "invite": [
                                   "@bob:ruma.test",
                                   "@carl:ruma.test"
                               ]}"#;

        let room_id = test.create_room_with_params(&alice_token, room_options);

        assert!(test.join_room(&alice_token, &room_id).status.is_success());
        assert!(test.join_room(&bob_token, &room_id).status.is_success());
        assert!(test.join_room(&carl_token, &room_id).status.is_success());
    }

    #[test]
    fn with_unknown_invited_users() {
        let test = Test::new();
        let alice_token = test.create_access_token_with_username("alice");
        test.create_access_token_with_username("bob");

        let room_options = r#"{"visibility": "private",
                               "invite": [
                                   "@bob:ruma.test",
                                   "@carl:ruma.test",
                                   "@dan:ruma.test"
                               ]}"#;

        let response = test.post(
            &format!( "/_matrix/client/r0/createRoom?access_token={}", alice_token),
            room_options
        );

        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_BAD_JSON"
        );

        let error = response.json().find("error").unwrap().as_str().unwrap().to_string();

        assert!(error.starts_with("Unknown users in invite list:"));
        assert!(error.contains("@carl:ruma.test"));
        assert!(error.contains("@dan:ruma.test"));
    }
}
