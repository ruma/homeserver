//! Endpoints for presence.

use bodyparser;
use iron::status::Status;
use iron::{Chain, Handler, IronResult, IronError, Plugin, Request, Response};
use ruma_identifiers::UserId;
use ruma_events::presence::PresenceState;
use time;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain, UserIdParam};
use modifier::SerializableResponse;
use models::room_membership::RoomMembership;
use models::presence_list::PresenceList;
use models::presence_status::PresenceStatus;
use models::user::User;

/// The PUT `/presence/:user_id/status` endpoint.
pub struct PutPresenceStatus;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PutPresenceStatusRequest {
    /// The status message to attach to this state.
    status_msg: Option<String>,
    /// The new presence state. One of: ["online", "offline", "unavailable"]
    presence: PresenceState,
}

middleware_chain!(PutPresenceStatus, [UserIdParam, JsonRequest, AccessTokenAuth]);

impl Handler for PutPresenceStatus {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let put_presence_status_request = match request.get::<bodyparser::Struct<PutPresenceStatusRequest>>() {
            Ok(Some(request)) => request,
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        if user_id != user.id {
            let error = ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string()
            );
            return Err(IronError::from(error));
        }

        PresenceStatus::upsert(
            &connection,
            &config.domain,
            &user_id,
            Some(put_presence_status_request.presence),
            put_presence_status_request.status_msg
        )?;

        Ok(Response::with(Status::Ok))
    }
}

/// The GET `/presence/:user_id/status` endpoint.
pub struct GetPresenceStatus;

middleware_chain!(GetPresenceStatus, [UserIdParam, AccessTokenAuth]);

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GetPresenceStatusResponse {
    /// The state message for this user if one was set.
    #[serde(skip_serializing_if = "Option::is_none")]
    status_msg: Option<String>,
    /// Whether the user is currently active.
    currently_active: bool,
    /// The length of time in milliseconds since an action was performed by this user.
    last_active_ago: u64,
    /// This user's presence. One of: ["online", "offline", "unavailable"]
    presence: PresenceState,
}

impl Handler for GetPresenceStatus {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;

        if user.id != user_id {
            let rooms = RoomMembership::find_common_joined_rooms(
                &connection,
                &user.id,
                &user_id,
                "join"
            )?;
            if rooms.is_empty() {
                Err(ApiError::unauthorized(
                    format!("You are not authorized to get the presence status for th given user_id: {}.", user_id)
                ))?;
            }
        }

        let status = match PresenceStatus::find_by_uid(&connection, &user_id)? {
            Some(status) => status,
            None => Err(
                ApiError::not_found("The given user_id does not correspond to an presence status".to_string())
            )?,
        };

        let presence_state: PresenceState = status.presence.parse()
            .expect("Database insert should ensure a PresenceState");

        let now = time::get_time();
        let last_update = time::Timespec::new(status.updated_at.0, 0);
        let last_active_ago: time::Duration = last_update - now;

        let response = GetPresenceStatusResponse {
            status_msg: status.status_msg,
            currently_active: PresenceState::Online == presence_state,
            last_active_ago: last_active_ago.num_milliseconds() as u64,
            presence: presence_state,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The POST `/presence/list/:user_id` endpoint.
pub struct PostPresenceList;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PostPresenceListRequest {
    /// A list of user IDs to remove from the list.
    invite: Vec<UserId>,
    /// A list of user IDs to add to the list.
    drop: Vec<UserId>,
}

middleware_chain!(PostPresenceList, [JsonRequest, UserIdParam, AccessTokenAuth]);

impl Handler for PostPresenceList {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let put_presence_list_request = match request.get::<bodyparser::Struct<PostPresenceListRequest>>() {
            Ok(Some(request)) => request,
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;

        if user_id != user.id {
            let error = ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string()
            );

            return Err(IronError::from(error));
        }

        PresenceList::update(
            &connection,
            &user_id,
            &put_presence_list_request.invite,
            put_presence_list_request.drop
        )?;

        Ok(Response::with(Status::Ok))
    }
}

/// The GET `/presence/list/:user_id` endpoint with response of `Vec<PresenceEvent>`.
pub struct GetPresenceList;

middleware_chain!(GetPresenceList, [UserIdParam, AccessTokenAuth]);

impl Handler for GetPresenceList {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let connection = DB::from_request(request)?;

        let (_, events) = PresenceList::find_events_by_uid(
            &connection,
            &user_id,
            None
        )?;

        Ok(Response::with((Status::Ok, SerializableResponse(events))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn basic_presence_status() {
        let test = Test::new();
        let alice = test.create_user();

        test.update_presence(&alice.token, &alice.id, r#"{"presence":"online"}"#);

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_status_path);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        Test::assert_json_keys(json, vec!["currently_active", "last_active_ago", "presence"]);
        assert_eq!(json.find("presence").unwrap().as_str().unwrap(), "online");
    }

    #[test]
    fn presence_status_message() {
        let test = Test::new();
        let alice = test.create_user();

        test.update_presence(&alice.token, &alice.id, r#"{"presence":"online", "status_msg": "Oscar!"}"#);

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_status_path);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        Test::assert_json_keys(json, vec!["currently_active", "last_active_ago", "presence", "status_msg"]);
        assert_eq!(json.find("presence").unwrap().as_str().unwrap(), "online");
        assert_eq!(json.find("status_msg").unwrap().as_str().unwrap(), "Oscar!");
    }

    #[test]
    fn forbidden_get_presence_status_no_shared_room() {
        let test = Test::new();
        let alice = test.create_user();
        let carl = test.create_user();

        test.update_presence(&alice.token, &alice.id, r#"{"presence":"online"}"#);

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            carl.token
        );
        let response = test.get(&presence_status_path);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn not_found_presence_status() {
        let test = Test::new();
        let alice = test.create_user();

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_status_path);
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn forbidden_put_presence_status() {
        let test = Test::new();
        let alice = test.create_user();
        let oscar = test.create_user();

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            oscar.token
        );
        let response = test.put(&presence_status_path, r#"{"presence":"online"}"#);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn basic_presence_list() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);
        let alice = test.create_user();
        let bob = test.create_user();
        let response = test.join_room(&alice.token, &room_id);
        assert_eq!(response.status, Status::Ok);
        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, &format!(r#"{{"invite":["{}", "{}"], "drop": []}}"#, carl.id, bob.id));
        assert_eq!(response.status, Status::Ok);

        let avatar_url_body = r#"{"avatar_url": "mxc://matrix.org/some/url"}"#;
        let avatar_url_path = format!(
            "/_matrix/client/r0/profile/{}/avatar_url?access_token={}",
            bob.id,
            bob.token
        );
        assert!(test.put(&avatar_url_path, avatar_url_body).status.is_success());

        test.update_presence(&bob.token, &bob.id, r#"{"presence":"online"}"#);
        test.update_presence(&bob.token, &bob.id, r#"{"presence":"online"}"#);
        test.update_presence(&carl.token, &carl.id, r#"{"presence":"online"}"#);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_list_path);
        assert_eq!(response.status, Status::Ok);
        let events = response.json().as_array().unwrap();
        println!("{:#?}", events);
        let mut events = events.into_iter();
        assert_eq!(events.len(), 2);

        assert_eq!(
            events.next().unwrap().find_path(&["content", "user_id"]).unwrap().as_str().unwrap(),
            bob.id
        );

        assert_eq!(
            events.next().unwrap().find_path(&["content", "user_id"]).unwrap().as_str().unwrap(),
            carl.id
        );
    }

    #[test]
    fn forbidden_presence_list_no_shared_room() {
        let test = Test::new();
        let alice = test.create_user();
        let carl = test.create_user();
        let bob = test.create_user();

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, &format!(r#"{{"invite":["{}", "{}"], "drop": []}}"#, carl.id, bob.id));
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn invitee_does_not_exist_presence_list() {
        let test = Test::new();
        let alice = test.create_user();

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, r#"{"invite":["@carl:ruma.test"], "drop": []}"#);
        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn to_dropped_does_not_exist_presence_list() {
        let test = Test::new();
        let alice = test.create_user();

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, r#"{"invite":[], "drop": ["@carl:ruma.test"]}"#);
        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn test_drop_presence_list() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);
        let bob = test.create_user();

        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, &format!(r#"{{"invite":["{}"], "drop": []}}"#, bob.id));
        assert_eq!(response.status, Status::Ok);

        test.update_presence(&bob.token, &bob.id, r#"{"presence":"online"}"#);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_list_path);
        assert_eq!(response.status, Status::Ok);
        let array = response.json().as_array().unwrap();
        assert_eq!(array.len(), 1);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, &format!(r#"{{"invite":[], "drop": ["{}"]}}"#, bob.id));
        assert_eq!(response.status, Status::Ok);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_list_path);
        assert_eq!(response.status, Status::Ok);
        let array = response.json().as_array().unwrap();
        assert_eq!(array.len(), 0);
    }
}
