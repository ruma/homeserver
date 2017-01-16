//! Endpoints for syncing.
use std::u64;
use std::error::Error;
use std::str::FromStr;

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use ruma_events::presence::PresenceState;
use serde_json::from_str;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, MiddlewareChain};
use models::user::User;
use modifier::SerializableResponse;
use query::{self, Batch, SyncOptions};

/// The `/sync` endpoint.
pub struct Sync;

middleware_chain!(Sync, [AccessTokenAuth]);

impl Handler for Sync {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let url = request.url.clone().into_generic_url();
        let query_pairs = url.query_pairs().into_owned();

        let mut filter = None;
        let mut since = None;
        let mut full_state = false;
        let mut set_presence = None;
        let mut timeout = 0;
        for tuple in query_pairs {
            match (tuple.0.as_ref(), tuple.1.as_ref()) {
                ("filter", value) => {
                    let content = from_str(value)
                        .map_err(|err| ApiError::invalid_param("filter", err.description()))?;
                    filter = Some(content);
                },
                ("since", value) => {
                    let batch = Batch::from_str(value)
                        .map_err(|err| ApiError::invalid_param("since", &err))?;
                    since = Some(batch);
                }
                ("full_state", "true") => {
                    full_state = true;
                }
                ("full_state", "false") => {
                    full_state = false;
                }
                ("full_state", _) => {
                    Err(ApiError::invalid_param("set_presence", "No boolean!"))?;
                }
                ("set_presence", "online") => {
                    set_presence = Some(PresenceState::Online);
                }
                ("set_presence", "unavailable") => {
                    set_presence = Some(PresenceState::Unavailable);
                }
                ("set_presence", "offline") => {
                    set_presence = Some(PresenceState::Offline);
                }
                ("set_presence", _) => {
                    Err(ApiError::invalid_param("set_presence", "Invalid enum!"))?;
                }
                ("timeout", value) => {
                    timeout = u64::from_str_radix(value, 10).map_err(|err| ApiError::invalid_param("timeout", err.description()))?;
                }
                _ => (),
            }
        }

        let options = SyncOptions {
            filter: filter,
            since: since,
            full_state: full_state,
            set_presence: set_presence,
            timeout: timeout,
        };

        let response = query::Sync::sync(&connection, &config.domain, &user, options)?;

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use test::Test;
    use iron::status::Status;
    use ruma_events::presence::PresenceState;
    use ruma_identifiers::EventId;
    use serde_json::from_str;

    use query::{SyncOptions};

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/03joined.pl#L3]
    #[test]
    fn can_sync_a_joined_room() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let next_batch = Test::get_next_batch(&response);
        let room = response.json().find_path(&["rooms", "join", room_id.as_ref()]).unwrap();
        assert!(room.is_object());

        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let room = response.json().find_path(&["rooms", "join", room_id.as_ref()]);
        assert_eq!(room, None);
    }

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/03joined.pl#L43]
    #[test]
    fn full_state_sync_includes_joined_rooms() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let since = Test::get_next_batch(&response);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: Some(since),
            full_state: true,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let room = response.json().find_path(&["rooms", "join", room_id.as_ref()]).unwrap();
        Test::assert_json_keys(room, vec!["timeline", "state", "ephemeral"]);
        Test::assert_json_keys(room.find("timeline").unwrap(), vec!["events", "limited", "prev_batch"]);
        Test::assert_json_keys(room.find("state").unwrap(), vec!["events"]);
        Test::assert_json_keys(room.find("ephemeral").unwrap(), vec!["events"]);
    }

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/03joined.pl#L81]
    #[test]
    fn newly_joined_room_is_included_in_an_incremental_sync() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let since = Test::get_next_batch(&response);
        let room_id = test.create_room(&carl.token);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: Some(since),
            full_state: true,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let since = Test::get_next_batch(&response);
        let room = response.json().find_path(&["rooms", "join", room_id.as_ref()]).unwrap();
        Test::assert_json_keys(room, vec!["timeline", "state", "ephemeral"]);
        Test::assert_json_keys(room.find("timeline").unwrap(), vec!["events", "limited", "prev_batch"]);
        Test::assert_json_keys(room.find("state").unwrap(), vec!["events"]);
        Test::assert_json_keys(room.find("ephemeral").unwrap(), vec!["events"]);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: Some(since),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let room = response.json().find_path(&["rooms", "join", room_id.as_ref()]);
        assert_eq!(room, None);
    }

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/04timeline.pl#L1]
    #[test]
    fn can_sync_a_room_with_a_single_message() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.send_message(&carl.token, &room_id, "Hi Test 1");
        assert_eq!(response.status, Status::Ok);
        let event_id_1 = response.json().find("event_id").unwrap().as_str().unwrap();

        let response = test.send_message(&carl.token, &room_id, "Hi Test 2");
        assert_eq!(response.status, Status::Ok);
        let event_id_2 = response.json().find("event_id").unwrap().as_str().unwrap();

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":2}}}"#).unwrap()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let json = response.json();
        let timeline = json.find_path(&["rooms", "join", room_id.as_ref(), "timeline"]).unwrap();
        assert_eq!(timeline.find("limited").unwrap().as_bool().unwrap(), true);
        let events = timeline.find("events").unwrap();
        assert!(events.is_array());
        let events = events.as_array().unwrap();
        assert_eq!(events.len(), 2);
        let mut events = events.into_iter();
        let event = events.next().unwrap();
        assert_eq!(EventId::try_from(event.find("event_id").unwrap().as_str().unwrap()).unwrap().opaque_id(), event_id_1);
        let event = events.next().unwrap();
        assert_eq!(EventId::try_from(event.find("event_id").unwrap().as_str().unwrap()).unwrap().opaque_id(), event_id_2);
    }

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/04timeline.pl#L223]
    #[test]
    fn syncing_a_new_room_with_a_large_timeline_limit_isnt_limited() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":100}}}"#).unwrap()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let json = response.json();
        let timeline = json.find_path(&["rooms", "join", room_id.as_ref(), "timeline"]).unwrap();
        assert_eq!(timeline.find("limited").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn initial_state() {
        let test = Test::new();
        let user = test.create_user();

        let sync_path = format!(
            "/_matrix/client/r0/sync?access_token={}",
            user.token,
        );

        let response = test.get(&sync_path);
        assert_eq!(response.status, Status::Ok);
        let rooms = response.json().find("rooms").unwrap();
        let join = rooms.find("join").unwrap();
        assert!(join.is_object());
        assert_eq!(join.as_object().unwrap().len(), 0);
    }

    #[test]
    fn initial_state_find_joined_rooms() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.send_message(&carl.token, &room_id, "Hi Test");
        assert_eq!(response.status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        let events = json.find_path(&["rooms", "join", room_id.as_ref(), "timeline", "events"]).unwrap();
        assert!(events.is_array());
        //TODO: This should be 6, but some unhandled events.
        assert_eq!(events.as_array().unwrap().len(), 3);
    }

    #[test]
    fn basic_since_state() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let next_batch = Test::get_next_batch(&response);

        test.send_message(&carl.token, &room_id, "test 1");
        assert_eq!(response.status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let json = response.json();
        let events = json.find_path(&["rooms", "join", room_id.as_ref(), "timeline", "events"]).unwrap();
        assert!(events.is_array());
        assert_eq!(events.as_array().unwrap().len(), 1);
    }

    #[test]
    fn set_presence() {
        let test = Test::new();
        let (alice, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        test.update_presence(&alice.token, &alice.id, r#"{"presence":"offline"}"#);

        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.get(&presence_status_path);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        Test::assert_json_keys(json, vec!["currently_active", "last_active_ago", "presence"]);
        assert_eq!(json.find("presence").unwrap().as_str().unwrap(), "offline");

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: Some(PresenceState::Online),
            timeout: 0
        };
        test.sync(&alice.token, options);

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
    fn basic_presence_state() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);
        let carl = test.create_user();
        let bob = test.create_user();
        let response = test.join_room(&carl.token, &room_id);
        assert_eq!(response.status, Status::Ok);
        let response = test.join_room(&bob.token, &room_id);
        assert_eq!(response.status, Status::Ok);

        let presence_list_path = format!(
            "/_matrix/client/r0/presence/list/{}?access_token={}",
            alice.id,
            alice.token
        );
        let response = test.post(&presence_list_path, &format!(r#"{{"invite":["{}", "{}"], "drop": []}}"#, bob.id, carl.id));
        assert_eq!(response.status, Status::Ok);

        test.update_presence(&bob.token, &bob.id, r#"{"presence":"online"}"#);
        test.update_presence(&carl.token, &carl.id, r#"{"presence":"online"}"#);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&alice.token, options);
        let array = response
            .json()
            .find("presence")
            .unwrap()
            .find("events")
            .unwrap()
            .as_array()
            .unwrap();
        let mut events = array.into_iter();
        assert_eq!(events.len(), 2);

        assert_eq!(
            events.next().unwrap().find_path(&["content", "user_id"]).unwrap().as_str().unwrap(),
            bob.id
        );

        assert_eq!(
            events.next().unwrap().find_path(&["content", "user_id"]).unwrap().as_str().unwrap(),
            carl.id
        );

        let next_batch = Test::get_next_batch(&response);
        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&alice.token, options);
        let array = response
            .json()
            .find("presence")
            .unwrap()
            .find("events")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(array.len(), 0);
    }

    #[test]
    fn invalid_since() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.get(&format!("/_matrix/client/r0/sync?since={}&access_token={}", "10s_234", carl.token));
        assert_eq!(response.status, Status::BadRequest);
    }

    #[test]
    fn invalid_timeout() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.get(&format!("/_matrix/client/r0/sync?timeout={}&access_token={}", "10s_234", carl.token));
        assert_eq!(response.status, Status::BadRequest);
    }

    #[test]
    fn invalid_set_presence() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.get(&format!("/_matrix/client/r0/sync?set_presence={}&access_token={}", "10s_234", carl.token));
        assert_eq!(response.status, Status::BadRequest);
    }

    #[test]
    fn invalid_filter() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.get(&format!("/_matrix/client/r0/sync?filter={}&access_token={}", "{10s_234", carl.token));
        assert_eq!(response.status, Status::BadRequest);
    }

    #[test]
    fn invalid_full_state() {
        let test = Test::new();
        let (carl, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.get(&format!("/_matrix/client/r0/sync?full_state={}&access_token={}", "{10s_234", carl.token));
        assert_eq!(response.status, Status::BadRequest);
    }
}
