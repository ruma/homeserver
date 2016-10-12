//! Endpoints for joining rooms.

use std::convert::TryFrom;
use std::error::Error;

use bodyparser;
use iron::status::Status;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use ruma_identifiers::UserId;

use config::Config;
use db::DB;
use error::{ApiError, MapApiError};
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain, RoomIdParam};
use modifier::SerializableResponse;
use room::Room;
use room_membership::{RoomMembership, RoomMembershipOptions};
use user::User;

/// The `/rooms/:room_id/join` endpoint.
pub struct JoinRoom;

#[derive(Debug, Serialize)]
struct JoinRoomResponse {
    room_id: String,
}

middleware_chain!(JoinRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for JoinRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("Should have been required by RoomIdParam.")
            .clone();

        let room_membership_options = RoomMembershipOptions {
            room_id: room_id.clone(),
            user_id: user.id.clone(),
            sender: user.id,
            membership: "join".to_string(),
        };

        let room_membership = RoomMembership::upsert(
            &connection,
            &config.domain,
            room_membership_options
        )?;

        let response = JoinRoomResponse { room_id: room_membership.room_id.to_string() };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The `/rooms/:room_id/invite` endpoint.
#[derive(Debug)]
pub struct InviteToRoom;

#[derive(Clone, Debug, Deserialize)]
struct InviteToRoomRequest {
    pub user_id: String,
}

middleware_chain!(InviteToRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for InviteToRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a room_id").clone();

        let inviter = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let invitee_id = match request.get::<bodyparser::Struct<InviteToRoomRequest>>() {
            Ok(Some(req)) => UserId::try_from(&req.user_id).map_api_err(|err| {
                ApiError::invalid_param("user_id", err.description())
            }),
            Ok(None) | Err(_) => Err(ApiError::missing_param("user_id"))
        }?;

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        // Check if the invitee exists.
        User::find_by_uid(&connection, &invitee_id)
            .map_err(IronError::from)?;

        // Check if the room exists.
        Room::find(&connection, &room_id)
            .map_err(IronError::from)?;

        // Check if the inviter has joined the room.
        let unauthorized_err = ApiError::unauthorized(
            Some("The inviter hasn't joined the room yet")
        );

        RoomMembership::find(&connection, &room_id, &inviter.id)
            .and_then(|membership| match membership {
                Some(entry) => match entry.membership.as_ref() {
                    "join" => Ok(()),
                    _ => Err(unauthorized_err)
                },
                None => Err(unauthorized_err)
            })?;

        let invitee_membership = RoomMembership::find(&connection, &room_id, &invitee_id)?;
        let new_membership_options = RoomMembershipOptions {
            room_id: room_id,
            user_id: invitee_id,
            sender: inviter.id,
            membership: "invite".to_string(),
        };

        match invitee_membership {
            Some(mut entry) => match entry.membership.as_ref() {
                "invite" => Ok(()),
                "ban" => Err(ApiError::unauthorized(
                    Some("The invited user is banned from the room")
                )),
                "join" => Err(ApiError::unauthorized(
                    Some("The invited user has already joined")
                )),
                _ => {
                    entry.update(
                        &connection,
                        &config.domain,
                        new_membership_options
                    )?;

                    Ok(())
                }
            },
            None => {
                RoomMembership::create(
                    &connection,
                    &config.domain,
                    new_membership_options
                )?;

                Ok(())
            }
        }?;

        Ok(Response::with(Status::Ok))
    }
}



#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn join_own_public_room() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_public_room(&access_token);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            access_token
        );

        let response = test.post(&room_join_path, r"{}");

        assert_eq!(response.status, Status::Ok);
        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_public_room() {
        let test = Test::new();
        let carl_token = test.create_access_token_with_username("carl");
        let mark_token = test.create_access_token_with_username("mark");

        let room_id = test.create_public_room(&carl_token);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            mark_token
        );

        let response = test.post(&room_join_path, r"{}");

        assert_eq!(response.status, Status::Ok);
        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_own_private_room() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_private_room(&access_token);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            access_token
        );

        let response = test.post(&room_join_path, r"{}");

        assert_eq!(response.status, Status::Ok);
        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_private_room() {
        let test = Test::new();
        let carl_token = test.create_access_token_with_username("carl");
        let mark_token = test.create_access_token_with_username("mark");

        let body = r#"{"visibility": "private", "invite": ["@mark:ruma.test"]}"#;
        let room_id = test.create_room_with_params(&carl_token, body);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            mark_token
        );

        let response = test.post(&room_join_path, r"{}");

        assert_eq!(response.status, Status::Ok);
        assert!(response.json().find("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_private_room_without_invite() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let alice_token = test.create_access_token_with_username("alice");

        let room_id = test.create_private_room(&bob_token);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            alice_token
        );

        let response = test.post(&room_join_path, r"{}");

        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn invite_to_room() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let alice_token = test.create_access_token_with_username("alice");

        let room_id = test.create_private_room(&bob_token);

        let response = test.invite(&bob_token, &room_id, "@alice:ruma.test");

        assert_eq!(response.status, Status::Ok);

        assert!(test.join_room(&alice_token, &room_id).status.is_success());
    }

    #[test]
    fn invite_before_joining() {
        let test = Test::new();

        let carl_token = test.create_access_token_with_username("carl");
        let bob_token = test.create_access_token_with_username("bob");
        let _ = test.create_access_token_with_username("alice");

        // Carl invites Bob.
        let body = r#"{"visibility": "private", "invite": ["@bob:ruma.test"]}"#;
        let room_id = test.create_room_with_params(&carl_token, body);

        // Bob invites Alice before joining.
        let response = test.invite(&bob_token, &room_id, "@alice:ruma.test");

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The inviter hasn't joined the room yet"
        );
    }

    #[test]
    fn invite_without_user_id() {
        let test = Test::new();
        let carl_token = test.create_access_token_with_username("carl");

        let room_id = test.create_private_room(&carl_token);
        let invite_path = format!(
            "/_matrix/client/r0/rooms/{}/invite?access_token={}",
            room_id,
            carl_token
        );

        // Empty body.
        let response = test.post(&invite_path, "{}");

        assert_eq!(response.status, Status::BadRequest);
        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_MISSING_PARAM"
        );
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "Missing value for required parameter: user_id."
        );
    }

    #[test]
    fn invitee_does_not_exist() {
        let test = Test::new();
        let carl_token = test.create_access_token_with_username("carl");

        let room_id = test.create_private_room(&carl_token);

        // User 'mark' does not exist.
        let response = test.invite(&carl_token, &room_id, "@mark:ruma.test");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The user @mark:ruma.test was not found on this server"
        );
    }

    #[test]
    fn invitee_is_invalid() {
        let test = Test::new();
        let carl_token = test.create_access_token();
        let room_id = test.create_private_room(&carl_token);

        let response = test.invite(&carl_token, &room_id, "mark.ruma.test");

        assert_eq!(response.status, Status::BadRequest);
        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "IO_RUMA_INVALID_PARAM"
        );
    }

    #[test]
    fn invitee_is_already_invited() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let _ = test.create_access_token_with_username("alice");

        let room_id = test.create_room_with_params(
            &bob_token,
            r#"{"visibility": "private", "invite": ["@alice:ruma.test"]}"#
        );

        let response = test.invite(&bob_token, &room_id, "@alice:ruma.test");

        assert_eq!(response.status, Status::Ok);
    }

    #[test]
    fn invitee_has_already_joined() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let alice_token = test.create_access_token_with_username("alice");

        let room_id = test.create_room_with_params(
            &bob_token,
            r#"{"visibility": "private", "invite": ["@alice:ruma.test"]}"#
        );

        assert!(test.join_room(&alice_token, &room_id).status.is_success());

        let response = test.invite(&bob_token, &room_id, "@alice:ruma.test");

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The invited user has already joined"
        );
    }

    #[test]
    fn room_does_not_exist() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let _ = test.create_access_token_with_username("alice");

        let response = test.invite(&bob_token, "!random:ruma.test", "@alice:ruma.test");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }
}
