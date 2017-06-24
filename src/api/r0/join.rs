//! Endpoints for joining rooms.

use std::error::Error;

use bodyparser;
use diesel::Connection;
use diesel::pg::PgConnection;
use iron::status::Status;
use iron::{Chain, Handler, IronResult, Plugin, Request, Response};
use ruma_identifiers::{UserId, RoomId, RoomIdOrAliasId};

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain, RoomIdParam, RoomIdOrAliasParam};
use models::room::Room;
use models::room_alias::RoomAlias;
use models::room_membership::{RoomMembership, RoomMembershipOptions};
use models::user::User;
use modifier::{SerializableResponse, EmptyResponse};


/// The `/rooms/:room_id/join` endpoint.
pub struct JoinRoom;

#[derive(Debug, Serialize)]
struct JoinRoomResponse {
    /// The joined room.
    room_id: RoomId,
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
fn join_room(room_id: RoomId, user: User, connection: &PgConnection, config: &Config) -> IronResult<Response> {
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

    let response = JoinRoomResponse { room_id: room_membership.room_id };

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
            Err(ApiError::unauthorized("The room was not found on this server".to_string()))?;
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
                        Ok(Response::with(EmptyResponse(Status::Ok)))
                    }
                    "ban" => {
                        Err(ApiError::unauthorized("User is banned from the room".to_string()))?
                    }
                    _ => Err(ApiError::unauthorized("Invalid membership state".to_string()))?,
                }
            }
            None => Err(ApiError::unauthorized("User not in room or uninvited".to_string()))?,
        }
    }
}


/// The `/rooms/:room_id/forget` endpoint.
pub struct ForgetRoom;

middleware_chain!(ForgetRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for ForgetRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        Ok(Response::with(EmptyResponse(Status::Ok)))
    }
}


#[derive(Clone, Debug, Deserialize)]
struct SenderActionRoomRequest {
    /// The reason the user has been kicked.
    #[serde(default)]
    pub reason: Option<String>,
    /// The fully qualified user ID of the user being kicked.
    pub user_id: UserId,
}

fn handle_membership(state: &str, request: &mut Request) -> IronResult<Response> {
    let room_id = request.extensions.get::<RoomIdParam>()
        .expect("RoomIdParam should ensure a room_id").clone();

    let sender = request.extensions.get::<User>()
        .expect("AccessTokenAuth should ensure a user").clone();
    let sender_id = sender.id;
    let user_id = match request.get::<bodyparser::Struct<SenderActionRoomRequest>>() {
        Ok(Some(req)) => req.user_id,
        Ok(None) => Err(ApiError::bad_json(None))?,
        Err(err) => Err(ApiError::bad_json(err.description().to_string()))?,
    };

    let connection = DB::from_request(request)?;
    let config = Config::from_request(request)?;

    connection.transaction::<(), ApiError, _>(|| {
        if User::find_active_user(&connection, &user_id)?.is_none() {
            Err(ApiError::not_found(format!("The user {} was not found on this server", user_id)))?;
        }

        let room = match Room::find(&connection, &room_id)? {
            Some(room) => room,
            None => Err(ApiError::unauthorized("The room was not found on this server".to_string()))?,
        };

        if !RoomMembership::find(&connection, &room_id, &sender_id)?
            .map(|m| m.membership == "join")
            .unwrap_or(false) {
            Err(ApiError::unauthorized("The sender is not currently in the room".to_string()))?
        }

        let membership = RoomMembership::find(&connection, &room_id, &user_id)?;

        let power_levels = room.current_power_levels(&connection)?;
        let user_power_level = power_levels
            .users
            .get(&sender_id)
            .unwrap_or(&power_levels.users_default);

        let mut room_membership_options = RoomMembershipOptions {
            room_id: room_id,
            user_id: user_id,
            sender: sender_id,
            membership: state.to_string(),
        };

        match state {
            "kick" => if power_levels.kick > *user_power_level {
                Err(ApiError::unauthorized("Insufficient power level to kick a user".to_string()))?;
            },
            "ban" => if power_levels.ban > *user_power_level {
                Err(ApiError::unauthorized("Insufficient power level to ban a user".to_string()))?;
            },
            "unban" => if power_levels.ban > *user_power_level {
                Err(ApiError::unauthorized("Insufficient power level to unban a user".to_string()))?;
            },
            "invite" => if power_levels.invite > *user_power_level {
                Err(ApiError::unauthorized("Insufficient power level to invite a user".to_string()))?;
            },
            _ => ()
        };

        match (state, membership) {
            ("kick", Some(mut membership)) => {
                room_membership_options.membership = "leave".to_string();
                if membership.membership.as_str() == "join" {
                    membership.update(&connection, &config.domain, room_membership_options)?;
                } else {
                    Err(ApiError::unauthorized("The user is not currently in the room".to_string()))?
                }
            }
            ("ban", Some(mut membership)) => {
                membership.update(&connection, &config.domain, room_membership_options)?;
            }
            ("unban", Some(mut membership)) => {
                room_membership_options.membership = "leave".to_string();
                if membership.membership.as_str() == "ban" {
                    membership.update(&connection, &config.domain, room_membership_options)?;
                } else {
                    Err(ApiError::unauthorized("The user is not banned in the room".to_string()))?
                }
            }
            ("invite", Some(mut membership)) => match membership.membership.as_ref() {
                "invite" => (),
                "ban" => Err(ApiError::unauthorized(
                    "The invited user is banned from the room".to_string()
                ))?,
                "join" => Err(ApiError::unauthorized(
                    "The invited user has already joined".to_string()
                ))?,
                _ => {
                    membership.update(&connection, &config.domain, room_membership_options)?;
                }
            },
            ("invite", None) => {
                RoomMembership::create(&connection, &config.domain, room_membership_options)?;
            }
            (_, None) => {
                Err(ApiError::unauthorized("The user is not currently in the room".to_string()))?
            }
            _ => ()
        }
        Ok(())
    })?;
    Ok(Response::with(EmptyResponse(Status::Ok)))
}


/// The `/rooms/:room_id/kick` endpoint.
pub struct KickFromRoom;

middleware_chain!(KickFromRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for KickFromRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        handle_membership("kick", request)
    }
}


/// The `/rooms/:room_id/ban` endpoint.
pub struct BanFromRoom;

middleware_chain!(BanFromRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for BanFromRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        handle_membership("ban", request)
    }
}


/// The `/rooms/:room_id/invite` endpoint.
pub struct InviteToRoom;

middleware_chain!(InviteToRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for InviteToRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        handle_membership("invite", request)
    }
}


/// The `/rooms/:room_id/unban` endpoint.
pub struct UnbanRoom;

middleware_chain!(UnbanRoom, [JsonRequest, RoomIdParam, AccessTokenAuth]);

impl Handler for UnbanRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        handle_membership("unban", request)
    }
}


#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn join_own_public_room_via_join_endpoint() {
        let test = Test::new();
        let user = test.create_user();
        let room_id = test.create_public_room(&user.token);

        let room_join_path = format!(
            "/_matrix/client/r0/join/{}?access_token={}",
            room_id,
            user.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("room_id").unwrap().as_str().unwrap().to_string(), room_id);
    }

    #[test]
    fn join_own_public_room_via_join_endpoint_alias() {
        let test = Test::new();
        let user = test.create_user();
        let room_id = test.create_room_with_params(
            &user.token,
            r#"{"room_alias_name":"thepub", "visibility": "public"}"#
        );

        let room_join_path = format!(
            "/_matrix/client/r0/join/{}?access_token={}",
            "%23thepub:ruma.test", // Hash symbols need to be urlencoded
            user.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("room_id").unwrap().as_str().unwrap().to_string(), room_id);
    }

    #[test]
    fn join_own_public_room() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            carl.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_public_room() {
        let test = Test::new();
        let (_, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);
        let mark = test.create_user();

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            mark.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_own_private_room() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            carl.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_private_room() {
        let test = Test::new();
        let carl = test.create_user();
        let mark = test.create_user();

        let body = format!(r#"{{"visibility": "private", "invite": ["{}"]}}"#, mark.id);
        let room_id = test.create_room_with_params(&carl.token, &body);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            mark.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Ok);
        assert!(response.json().get("room_id").unwrap().as_str().is_some());
    }

    #[test]
    fn join_other_private_room_without_invite() {
        let test = Test::new();
        let (_, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);
        let alice = test.create_user();

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            alice.token
        );

        let response = test.post(&room_join_path, r"{}");
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn invite_to_room() {
        let test = Test::new();
        let (bob, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);
        let alice = test.create_user();

        let response = test.invite(&bob.token, &room_id, &alice.id);
        assert_eq!(response.status, Status::Ok);

        assert_eq!(test.join_room(&alice.token, &room_id).status, Status::Ok);
    }

    #[test]
    fn invite_before_joining() {
        let test = Test::new();

        let carl = test.create_user();
        let bob = test.create_user();
        let alice = test.create_user();

        // Carl invites Bob.
        let body = format!(r#"{{"visibility": "private", "invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&carl.token, &body);

        // Bob invites Alice before joining.
        let response = test.invite(&bob.token, &room_id, &alice.id);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The sender is not currently in the room"
        );
    }

    #[test]
    fn invite_without_user_id() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let invite_path = format!(
            "/_matrix/client/r0/rooms/{}/invite?access_token={}",
            room_id,
            carl.token
        );

        // Empty body.
        let response = test.post(&invite_path, "{}");
        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn invitee_does_not_exist() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        // User 'mark' does not exist.
        let response = test.invite(&carl.token, &room_id, "@mark:ruma.test");
        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The user @mark:ruma.test was not found on this server"
        );
    }

    #[test]
    fn invitee_is_invalid() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let response = test.invite(&carl.token, &room_id, "mark.ruma.test");
        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn invitee_is_already_invited() {
        let test = Test::new();
        let bob = test.create_user();
        let alice = test.create_user();

        let room_id = test.create_room_with_params(
            &bob.token,
            format!(r#"{{"visibility": "private", "invite": ["{}"]}}"#, alice.id).as_str());

        let response = test.invite(&bob.token, &room_id, &alice.id);
        assert_eq!(response.status, Status::Ok);
    }

    #[test]
    fn invitee_has_already_joined() {
        let test = Test::new();
        let bob = test.create_user();
        let alice = test.create_user();

        let room_id = test.create_room_with_params(
            &bob.token,
            format!(r#"{{"visibility": "private", "invite": ["{}"]}}"#, alice.id).as_str());

        assert!(test.join_room(&alice.token, &room_id).status.is_success());

        let response = test.invite(&bob.token, &room_id, &alice.id);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The invited user has already joined"
        );
    }

    #[test]
    fn room_does_not_exist() {
        let test = Test::new();
        let bob = test.create_user();
        let alice = test.create_user();

        let response = test.invite(&bob.token, "!random:ruma.test", &alice.id);

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }

    #[test]
    fn leave_own_room() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            alice.token
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&alice.token, &room_id, "Hi", 1);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            format!("The user {} has not joined the room", alice.id)
        );
    }

    #[test]
    fn leave_nonexistent_room() {
        let test = Test::new();
        let alice = test.create_user();

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            "!random_room_id:ruma.test",
            alice.token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }

    #[test]
    fn leave_uninvited_room() {
        let test = Test::new();
        let bob = test.create_user();
        let (_, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob.token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "User not in room or uninvited"
        );
    }

    #[test]
    fn leave_invited_room() {
        let test = Test::new();
        let bob = test.create_user();
        let (alice, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let response = test.invite(&alice.token, &room_id, &bob.id);
        assert_eq!(response.status, Status::Ok);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob.token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&bob.token, &room_id, "Hi", 1);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            format!("The user {} has not joined the room", bob.id)
        );
    }

    #[test]
    fn leave_joined_room() {
        let test = Test::new();
        let bob = test.create_user();
        let (alice, room_id) = test.initial_fixtures(r#"{"visibility": "private"}"#);

        let response = test.invite(&alice.token, &room_id, &bob.id);
        assert_eq!(response.status, Status::Ok);

        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let leave_room_path = format!(
            "/_matrix/client/r0/rooms/{}/leave?access_token={}",
            room_id,
            bob.token,
        );

        let response = test.post(&leave_room_path, r#"{}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.send_message(&bob.token, &room_id, "Hi", 1);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            format!("The user {} has not joined the room", bob.id)
        );
    }

    #[test]
    fn kick_user() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{"invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        assert_eq!(
            test.send_message(&bob.token, &room_id, "Hi", 1).status,
            Status::Ok
        );

        assert_eq!(
            test.kick_from_room(&alice.token, &room_id, &bob.id, None).status,
            Status::Ok
        );

        let response = test.send_message(&bob.token, &room_id, "Hi", 2);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            format!("The user {} has not joined the room", &bob.id)
        );
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn kick_user_without_permissions() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{"invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        assert_eq!(
            test.send_message(&bob.token, &room_id, "Hi", 1).status,
            Status::Ok
        );

        let response = test.kick_from_room(&bob.token, &room_id, &alice.id, None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "Insufficient power level to kick a user"
        );
    }

    #[test]
    fn kick_user_from_invalid_room() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{"invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        assert_eq!(
            test.send_message(&bob.token, &room_id, "Hi", 1).status,
            Status::Ok
        );

        let response = test.kick_from_room(&alice.token, "!invalid_room:ruma.test", &bob.id, None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The room was not found on this server"
        );
    }

    #[test]
    fn kicker_not_in_room() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();
        let carl = test.create_user();

        let room_options = format!(r#"{{"invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let response = test.kick_from_room(&bob.token, &room_id, &carl.id, None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The sender is not currently in the room"
        );
    }

    #[test]
    fn kickee_not_in_room() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{"invite": ["{}"]}}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let response = test.kick_from_room(&alice.token, &room_id, &bob.id, None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The user is not currently in the room"
        );
    }
}
