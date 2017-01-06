//! Endpoints for joining rooms.

use std::convert::TryFrom;
use std::error::Error;

use bodyparser;
use diesel::Connection;
use diesel::pg::PgConnection;
use iron::status::Status;
use iron::{Chain, Handler, IronResult, IronError, Plugin, Request, Response};
use ruma_identifiers::{UserId, RoomId, RoomIdOrAliasId};

use config::Config;
use db::DB;
use error::{ApiError, MapApiError};
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain, RoomIdParam, RoomIdOrAliasParam};
use modifier::SerializableResponse;
use models::room::Room;
use models::room_alias::RoomAlias;
use models::room_membership::{RoomMembership, RoomMembershipOptions};
use models::user::User;


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

        join_room(room_id, user, &connection, &config)
    }
}

/// The `/join/:room_id_or_alias` endpoint.
pub struct JoinRoomWithIdOrAlias;

middleware_chain!(JoinRoomWithIdOrAlias, [JsonRequest, RoomIdOrAliasParam, AccessTokenAuth]);

impl Handler for JoinRoomWithIdOrAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let room_id_or_alias = request.extensions.get::<RoomIdOrAliasParam>()
            .expect("Should have been required by RoomIdOrAliasParam.")
            .clone();

        let room_id = match room_id_or_alias {
            RoomIdOrAliasId::RoomId(id) => id,
            RoomIdOrAliasId::RoomAliasId(alias) => {
                let room_alias = RoomAlias::find_by_alias(&connection, &alias)?;
                room_alias.room_id
            }
        };

        join_room(room_id, user, &connection, &config)
    }
}

/// Handles the work of actually saving the user to the room membership table
fn join_room(room_id: RoomId, user: User, connection: &PgConnection, config: &Config
    ) -> IronResult<Response> {
    let room_membership_options = RoomMembershipOptions {
        room_id: room_id.clone(),
        user_id: user.id.clone(),
        sender: user.id,
        membership: "join".to_string(),
    };

    let room_membership = RoomMembership::upsert(
        connection,
        &config.domain,
        room_membership_options
    )?;

    let response = JoinRoomResponse { room_id: room_membership.room_id.to_string() };

    Ok(Response::with((Status::Ok, SerializableResponse(response))))
}

/// The `/rooms/:room_id/leave` endpoint.
pub struct LeaveRoom;

middleware_chain!(LeaveRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for LeaveRoom {
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
            sender: user.id.clone(),
            membership: "leave".to_string(),
        };

        if Room::find(&connection, &room_id)?.is_none() {
            return Err(IronError::from(ApiError::unauthorized("The room was not found on this server".to_string())));
        }

        match RoomMembership::find(&connection, &room_id, &user.id)? {
            Some(mut room_membership) => {
                match room_membership.membership.as_str() {
                    "leave" => Ok(Response::with(Status::Ok)),
                    "join" | "invite" => {
                        room_membership.update(
                            &connection,
                            &config.domain,
                            room_membership_options)?;
                        Ok(Response::with((Status::Ok)))
                    },
                    "ban" => {
                        Err(IronError::from(ApiError::unauthorized("User is banned from the room".to_string())))
                    },
                    _ => Err(IronError::from(ApiError::unauthorized("Invalid membership state".to_string())))
                }
            },
            None => Err(IronError::from(ApiError::unauthorized("User not in room or uninvited".to_string()))),
        }
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

        let invitee_membership = connection.transaction::<Option<RoomMembership>, ApiError, _>(|| {
            if User::find_active_user(&connection, &invitee_id)?.is_none() {
                return Err(
                    ApiError::not_found(format!("The invited user {} was not found on this server", invitee_id))
                );
            }

            if Room::find(&connection, &room_id)?.is_none() {
                return Err(
                    ApiError::unauthorized("The room was not found on this server".to_string())
                );
            }

            let unauthorized_err = ApiError::unauthorized(
                "The inviter hasn't joined the room yet".to_string()
            );

            // Check if the inviter has joined the room.
            RoomMembership::find(&connection, &room_id, &inviter.id)
                .and_then(|membership| match membership {
                    Some(entry) => match entry.membership.as_ref() {
                        "join" => Ok(()),
                        _ => Err(unauthorized_err)
                    },
                    None => Err(unauthorized_err)
                })?;

            let membership = RoomMembership::find(&connection, &room_id, &invitee_id)?;

            Ok(membership)
        }).map_err(ApiError::from)?;

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
                    "The invited user is banned from the room".to_string()
                )),
                "join" => Err(ApiError::unauthorized(
                    "The invited user has already joined".to_string()
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
    fn join_own_public_room_via_join_endpoint() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_public_room(&access_token);

        let room_join_path = format!(
            "/_matrix/client/r0/join/{}?access_token={}",
            room_id,
            access_token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().find("room_id").unwrap().as_str().unwrap().to_string(), room_id);
    }

    #[test]
    fn join_own_public_room_via_join_endpoint_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room_with_params(
            &access_token,
            r#"{"room_alias_name":"thepub", "visibility": "public"}"#
        );

        let room_join_path = format!(
            "/_matrix/client/r0/join/{}?access_token={}",
            "%23thepub:ruma.test", // Hash symbols need to be urlencoded
            access_token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().find("room_id").unwrap().as_str().unwrap().to_string(), room_id);
    }

    #[test]
    fn join_own_public_room() {
        let test = Test::new();
        let (access_token, room_id) = test.initial_fixtures("carl", r#"{"visibility": "public"}"#);

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
        let (_, room_id) = test.initial_fixtures("carl", r#"{"visibility": "public"}"#);
        let mark_token = test.create_access_token_with_username("mark");

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
        let (access_token, room_id) = test.initial_fixtures("carl", r#"{"visibility": "private"}"#);

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
        let (_, room_id) = test.initial_fixtures("bob", r#"{"visibility": "private"}"#);
        let alice_token = test.create_access_token_with_username("alice");

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
        let (bob_token, room_id) = test.initial_fixtures("bob", r#"{"visibility": "private"}"#);
        let alice_token = test.create_access_token_with_username("alice");

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
        let (carl_token, room_id) = test.initial_fixtures("carl", r#"{"visibility": "private"}"#);

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
        let (carl_token, room_id) = test.initial_fixtures("carl", r#"{"visibility": "private"}"#);

        // User 'mark' does not exist.
        let response = test.invite(&carl_token, &room_id, "@mark:ruma.test");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The invited user @mark:ruma.test was not found on this server"
        );
    }

    #[test]
    fn invitee_is_invalid() {
        let test = Test::new();
        let (carl_token, room_id) = test.initial_fixtures("carl", r#"{"visibility": "private"}"#);

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

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }

    #[test]
    fn leave_own_room() {
        let test = Test::new();
        let (alice_token, room_id) = test.initial_fixtures("alice", r#"{"visibility": "private"}"#);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            alice_token
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&alice_token, &room_id, "Hi");
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The user @alice:ruma.test has not joined the room"
        );
    }

    #[test]
    fn leave_nonexistent_room() {
        let test = Test::new();
        let alice_token = test.create_access_token_with_username("alice");

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            "!random_room_id:ruma.test",
            alice_token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }

    #[test]
    fn leave_uninvited_room() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let (_, room_id) = test.initial_fixtures("alice", r#"{"visibility": "public"}"#);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob_token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "User not in room or uninvited"
        );
    }

    #[test]
    fn leave_invited_room() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let (alice_id, room_id) = test.initial_fixtures("alice", r#"{"visibility": "private"}"#);

        let response = test.invite(&alice_id, &room_id, "@bob:ruma.test");
        assert_eq!(response.status, Status::Ok);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob_token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&bob_token, &room_id, "Hi");
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The user @bob:ruma.test has not joined the room"
        );
    }

    #[test]
    fn leave_joined_room() {
        let test = Test::new();
        let bob_token = test.create_access_token_with_username("bob");
        let (alice_id, room_id) = test.initial_fixtures("alice", r#"{"visibility": "private"}"#);

        let response = test.invite(&alice_id, &room_id, "@bob:ruma.test");
        assert_eq!(response.status, Status::Ok);

        let response = test.join_room(&bob_token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob_token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&bob_token, &room_id, "Hi");
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "The user @bob:ruma.test has not joined the room"
        );
    }
}
