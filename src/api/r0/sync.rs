//! Endpoints for syncing.
use std::u64;
use std::error::Error;
use std::str::FromStr;

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use ruma_events::presence::PresenceState;
use serde_json::from_str;
use url::Url;

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

        let url: Url = request.url.clone().into();
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

    use models::filter::ContentFilter;
    use query::{SyncOptions};

    #[test]
    fn sync_without_new_events() {
        let test = Test::new();
        let (alice, _) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: Some(PresenceState::Online),
            timeout: 0
        };

        let response = test.sync(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let first_batch = Test::get_next_batch(&response);

        // Sync again without any new events.
        // The next_batch token should be the same.
        let options = SyncOptions {
            filter: None,
            since: Some(first_batch.clone()),
            full_state: false,
            set_presence: None,
            timeout: 0
        };

        let response = test.sync(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let second_batch = Test::get_next_batch(&response);

        assert_eq!(first_batch, second_batch);
    }

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
        let room = response.json().pointer(&format!("/rooms/join/{}", room_id)).unwrap();
        assert!(room.is_object());

        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let room = response.json().pointer(&format!("/rooms/join/{}", room_id));
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
        let room = response.json().pointer(&format!("/rooms/join/{}", room_id)).unwrap();
        Test::assert_json_keys(room, vec!["timeline", "state", "ephemeral"]);
        Test::assert_json_keys(room.get("timeline").unwrap(), vec!["events", "limited", "prev_batch"]);
        Test::assert_json_keys(room.get("state").unwrap(), vec!["events"]);
        Test::assert_json_keys(room.get("ephemeral").unwrap(), vec!["events"]);
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
        let room = response.json().pointer(&format!("/rooms/join/{}", room_id)).unwrap();
        Test::assert_json_keys(room, vec!["timeline", "state", "ephemeral"]);
        Test::assert_json_keys(room.get("timeline").unwrap(), vec!["events", "limited", "prev_batch"]);
        Test::assert_json_keys(room.get("state").unwrap(), vec!["events"]);
        Test::assert_json_keys(room.get("ephemeral").unwrap(), vec!["events"]);

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":10}}}"#).unwrap()),
            since: Some(since),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let room = response.json().pointer(&format!("/rooms/join/{}", room_id));
        assert_eq!(room, None);
    }

    /// [https://github.com/matrix-org/sytest/blob/0eba37fc567d65f0a005090548c8df4d0e43775f/tests/31sync/04timeline.pl#L1]
    #[test]
    fn can_sync_a_room_with_a_single_message() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.send_message(&carl.token, &room_id, "Hi Test 1", 1);
        assert_eq!(response.status, Status::Ok);
        let event_id_1 = response.json().get("event_id").unwrap().as_str().unwrap();

        let response = test.send_message(&carl.token, &room_id, "Hi Test 2", 2);
        assert_eq!(response.status, Status::Ok);
        let event_id_2 = response.json().get("event_id").unwrap().as_str().unwrap();

        let options = SyncOptions {
            filter: Some(from_str(r#"{"room":{"timeline":{"limit":2}}}"#).unwrap()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let timeline = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline", room_id))
            .unwrap();
        assert_eq!(timeline.get("limited").unwrap().as_bool().unwrap(), true);
        let events = timeline.get("events").unwrap();
        assert!(events.is_array());
        let events = events.as_array().unwrap();
        assert_eq!(events.len(), 2);
        let mut events = events.into_iter();
        let event = events.next().unwrap();
        assert_eq!(EventId::try_from(event.get("event_id").unwrap().as_str().unwrap()).unwrap().opaque_id(), event_id_1);
        let event = events.next().unwrap();
        assert_eq!(EventId::try_from(event.get("event_id").unwrap().as_str().unwrap()).unwrap().opaque_id(), event_id_2);
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
        let timeline = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline", room_id))
            .unwrap();
        assert_eq!(timeline.get("limited").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn sync_joined_room_state() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{
            "invite": ["{}"],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.topic",
                "content": {{ "topic": "Initial Topic" }}
            }}]
        }}"#, bob.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);
        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let response = test.sync(&alice.token, options.clone());
        let alice_next_batch = Test::get_next_batch(&response);
        assert_eq!(response.status, Status::Ok);

        let state_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        for e in state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.create" => {
                    assert_eq!(
                        e.pointer("/content/creator").unwrap().as_str().unwrap(),
                        alice.id
                    );
                },
                "m.room.history_visibility" => {
                    assert_eq!(
                        e.pointer("/content/history_visibility").unwrap().as_str().unwrap(),
                        "shared"
                    );
                },
                "m.room.member" => {
                    assert_eq!(
                        e.get("sender").unwrap().as_str().unwrap(),
                        bob.id
                    );

                    assert_eq!(
                        e.pointer("/content/membership").unwrap().as_str().unwrap(),
                        "join"
                    );
                },
                "m.room.power_levels" => {
                    assert_eq!(
                        e.pointer("/content/kick").unwrap().as_u64().unwrap(),
                        50
                    );
                },
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Initial Topic"
                    );
                },
                _ => {}
            }
        }

        let response = test.send_state_event(
            &alice.token,
            &room_id,
            "m.room.topic",
            r#"{ "topic": "Updated Topic" }"#,
            None
        );
        assert_eq!(response.status, Status::Ok);

        let response = test.sync(&bob.token, options.clone());
        assert_eq!(response.status, Status::Ok);

        let state_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert_eq!(state_events.len(), 6);

        for e in state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Updated Topic"
                    );
                },
                _ => {}
            }
        }

        let options = SyncOptions {
            filter: None,
            since: Some(alice_next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let response = test.sync(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let state_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert_eq!(state_events.len(), 1);
        assert_eq!(
            state_events[0].get("type").unwrap().as_str().unwrap(),
            "m.room.topic"
        );
        assert_eq!(
            state_events[0].pointer("/content/topic").unwrap().as_str().unwrap(),
            "Updated Topic"
        );
    }

    #[test]
    fn sync_invited_room_state() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{
            "invite": ["{}"],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.name",
                "content": {{ "name": "Initial Name" }}
            }}]
        }}"#, bob.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let alice_sync_response = test.sync(&alice.token, options.clone());
        assert_eq!(alice_sync_response.status, Status::Ok);

        let joined_state_events = alice_sync_response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(joined_state_events.len() > 0);

        for e in joined_state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.name" => {
                    assert_eq!(
                        e.pointer("/content/name").unwrap().as_str().unwrap(),
                        "Initial Name"
                    );
                },
                _ => {}
            }
        }

        let bob_sync_response = test.sync(&bob.token, options.clone());
        let bob_next_batch = Test::get_next_batch(&bob_sync_response);
        assert_eq!(bob_sync_response.status, Status::Ok);

        let invited_state_events = bob_sync_response
            .json()
            .pointer(&format!("/rooms/invite/{}/invite_state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(invited_state_events.len() > 0);

        for e in invited_state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.name" => {
                    assert_eq!(
                        e.pointer("/content/name").unwrap().as_str().unwrap(),
                        "Initial Name"
                    );
                },
                "m.room.create" => {
                    assert_eq!(
                        e.pointer("/content/creator").unwrap().as_str().unwrap(),
                        alice.id
                    );
                }
                _ => {}
            }
        }

        let room_name_event_response = test.send_state_event(
            &alice.token,
            &room_id,
            "m.room.name",
            r#"{ "name": "Updated Name" }"#,
            None
        );
        assert_eq!(room_name_event_response.status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: Some(bob_next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let bob_sync_response = test.sync(&bob.token, options.clone());
        assert_eq!(bob_sync_response.status, Status::Ok);

        let invited_state_events = bob_sync_response
            .json()
            .pointer(&format!("/rooms/invite/{}/invite_state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(invited_state_events.len() > 1);

        for e in invited_state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.name" => {
                    assert_eq!(
                        e.pointer("/content/name").unwrap().as_str().unwrap(),
                        "Updated Name"
                    );
                },
                _ => {},
            }
        }
    }

    #[test]
    fn sync_left_room_state() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{ "invite": ["{}"] }}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let bob_sync_response = test.sync(&bob.token, options.clone());
        let bob_next_batch = Test::get_next_batch(&bob_sync_response);
        assert_eq!(bob_sync_response.status, Status::Ok);

        let state_events = bob_sync_response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(state_events.len() > 0);

        assert_eq!(test.leave_room(&bob.token, &room_id).status, Status::Ok);

        let response = test.send_state_event(
            &alice.token,
            &room_id,
            "m.room.topic",
            r#"{ "topic": "New Topic" }"#,
            None
        );
        assert_eq!(response.status, Status::Ok);

        // Bob syncs and uses a custom filter to include the left rooms in the response.
        let include_leave_filter: ContentFilter = from_str(r#"{"room":{"include_leave":true}}"#).unwrap();

        let options = SyncOptions {
            filter: Some(include_leave_filter.clone()),
            since: Some(bob_next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let bob_sync_response = test.sync(&bob.token, options.clone());
        let bob_next_batch = Test::get_next_batch(&bob_sync_response);
        assert_eq!(bob_sync_response.status, Status::Ok);

        let left_state_events = bob_sync_response
            .json()
            .pointer(&format!("/rooms/leave/{}/state/events", room_id))
            .unwrap()
            .as_array()
            .unwrap();

        for e in left_state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "New Topic"
                    );
                },
                _ => { },
            }
        }

        let response = test.send_state_event(
            &alice.token,
            &room_id,
            "m.room.topic",
            r#"{ "topic": "Another Topic" }"#,
            None
        );
        assert_eq!(response.status, Status::Ok);

        // Bob syncs with the default settings. i.e without a custom filter.
        let options = SyncOptions {
            filter: None,
            since: Some(bob_next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let bob_sync_response = test.sync(&bob.token, options.clone());
        assert_eq!(bob_sync_response.status, Status::Ok);

        let leave_rooms = bob_sync_response
            .json()
            .pointer("/rooms/leave").unwrap()
            .as_object().unwrap();
        assert_eq!(leave_rooms.len(), 0);

        // Sync with the custom filter to include the left rooms.
        // Bob can't access the latest state changes because he left the room.
        let options = SyncOptions {
            filter: Some(include_leave_filter.clone()),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let bob_sync_response = test.sync(&bob.token, options.clone());
        assert_eq!(bob_sync_response.status, Status::Ok);

        let left_state_events = bob_sync_response
            .json()
            .pointer(&format!("/rooms/leave/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        for e in left_state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "New Topic"
                    );
                },
                _ => { },
            }
        }
    }

    #[test]
    fn sync_left_room_timeline() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{ "invite": ["{}"] }}"#, bob.id);
        let room_id = test.create_room_with_params(&alice.token, &room_options);
        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        // Alice sends a message visible by Bob.
        assert_eq!(test.send_message(&alice.token, &room_id, "Hi Bob", 1).status, Status::Ok);

        // Bob leaves the room and can no longer receive new messages in his timeline.
        assert_eq!(test.leave_room(&bob.token, &room_id).status, Status::Ok);

        // Alice sends a message not visible to Bob.
        assert_eq!(test.send_message(&alice.token, &room_id, "Goodbye Bob", 2).status, Status::Ok);

        // Full timeline sync up to the point that Bob left the room.
        let include_leave_filter = from_str(r#"{"room":{"include_leave":true}}"#).unwrap();
        let options = SyncOptions {
            filter: Some(include_leave_filter),
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let response = test.sync(&bob.token, options);
        assert_eq!(response.status, Status::Ok);

        let left_room_timeline_events = response
            .json()
            .pointer(&format!("/rooms/leave/{}/timeline/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(left_room_timeline_events.len() > 1);

        let last_room_event = left_room_timeline_events.last().unwrap();
        assert_eq!(last_room_event.get("type").unwrap().as_str().unwrap(), "m.room.message");
        assert_eq!(last_room_event.get("sender").unwrap().as_str().unwrap(), alice.id);
        assert_eq!(last_room_event.pointer("/content/body").unwrap().as_str().unwrap(), "Hi Bob");
    }

    #[test]
    fn full_state() {
        let test = Test::new();
        let alice = test.create_user();

        let room_options = r#"{
            "initial_state": [{
                "state_key": "",
                "type": "m.room.topic",
                "content": { "topic": "Initial Topic" }
            }, {
                "state_key": "",
                "type": "m.room.name",
                "content": { "name": "Initial Name" }
            }]
        }"#;

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let options = SyncOptions {
            filter: None,
            since: None,
            full_state: false,
            set_presence: None,
            timeout: 0,
        };

        let response = test.sync(&alice.token, options);
        let next_batch = Test::get_next_batch(&response);
        assert_eq!(response.status, Status::Ok);

        let state_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(state_events.len() > 0);

        let timeline_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline/events", room_id)).unwrap()
            .as_array().unwrap();

        assert!(timeline_events.len() > 0);

        let initial_state_events_len = state_events.len();
        assert!(initial_state_events_len > 0);

        // Sync without any timeline events and with all the state events.
        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: true,
            set_presence: None,
            timeout: 0,
        };

        let response = test.sync(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let state_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/state/events", room_id)).unwrap()
            .as_array().unwrap();

        assert_eq!(state_events.len(), initial_state_events_len);

        for e in state_events.iter() {
            let event_type = e.get("type").unwrap().as_str().unwrap();

            match event_type {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Initial Topic"
                    );
                },
                "m.room.name" => {
                    assert_eq!(
                        e.pointer("/content/name").unwrap().as_str().unwrap(),
                        "Initial Name"
                    );
                },
                _ => { }
            }
        }

        let timeline_events = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline/events", room_id)).unwrap()
            .as_array().unwrap();

        assert_eq!(timeline_events.len(), 0);
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
        let rooms = response.json().get("rooms").unwrap();
        let join = rooms.get("join").unwrap();
        assert!(join.is_object());
        assert_eq!(join.as_object().unwrap().len(), 0);
    }

    #[test]
    fn initial_state_find_joined_rooms() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let response = test.send_message(&carl.token, &room_id, "Hi Test", 1);
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
        let events = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline/events", room_id))
            .unwrap();
        assert!(events.is_array());
        assert_eq!(events.as_array().unwrap().len(), 6);
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

        test.send_message(&carl.token, &room_id, "test 1", 1);
        assert_eq!(response.status, Status::Ok);

        let options = SyncOptions {
            filter: None,
            since: Some(next_batch),
            full_state: false,
            set_presence: None,
            timeout: 0
        };
        let response = test.sync(&carl.token, options);
        let events = response
            .json()
            .pointer(&format!("/rooms/join/{}/timeline/events", room_id))
            .unwrap();
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
        assert_eq!(json.get("presence").unwrap().as_str().unwrap(), "offline");

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
        assert_eq!(json.get("presence").unwrap().as_str().unwrap(), "online");
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
        let array = response.json().pointer("/presence/events").unwrap().as_array().unwrap();
        let mut events = array.into_iter();
        assert_eq!(events.len(), 2);

        assert_eq!(
            events.next().unwrap().pointer("/content/user_id").unwrap().as_str().unwrap(),
            bob.id
        );

        assert_eq!(
            events.next().unwrap().pointer("/content/user_id").unwrap().as_str().unwrap(),
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
        let array = response.json().pointer("/presence/events").unwrap().as_array().unwrap();
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
