//! Endpoint for retrieving the state of a room.

use std::convert::TryInto;

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use ruma_events::collections::all::StateEvent;

use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, MiddlewareChain, RoomIdParam};
use models::event::Event;
use models::room::Room;
use models::room_membership::RoomMembership;
use models::user::User;
use modifier::SerializableResponse;

/// The `/rooms/:room_id/state` endpoint.
pub struct RoomState;

middleware_chain!(RoomState, [RoomIdParam, AccessTokenAuth]);

impl Handler for RoomState {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a room_id").clone();

        let connection = DB::from_request(request)?;

        let room = match Room::find(&connection, &room_id)? {
            Some(room) => room,
            None => {
                Err(ApiError::unauthorized("The room was not found on this server".to_string()))?
            }
        };

        let membership = RoomMembership::find(&connection, &room.id, &user.id)?;

        if membership.is_none() {
            Err(ApiError::unauthorized("The user is not a member of the room".to_string()))?
        }

        let membership_state = membership.clone().unwrap().membership;
        let mut events = Vec::<Event>::new();

        match membership_state.as_ref() {
            "join" => {
                events.append(
                    &mut Event::get_room_full_state(&connection, &room_id)?
                );
            },
            "leave" => {
                let last_event = Event::find(&connection, &membership.unwrap().event_id)?
                    .expect("A room membership should be associated with an event");

                events.append(
                    &mut Event::get_room_state_events_until(&connection, &room_id, &last_event)?
                );
            },
            _ => {}
        }

        let mut state_events: Vec<StateEvent> = Vec::new();

        for event in events {
            state_events.push(event.try_into()?);
        }

        Ok(Response::with((Status::Ok, SerializableResponse(state_events))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;
    use serde_json::Value;

    #[test]
    fn forbidden_for_non_members() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_id = test.create_public_room(&alice.token);

        let room_state_path = format!(
            "/_matrix/client/r0/rooms/{}/state?access_token={}",
            room_id,
            bob.token
        );

        assert_eq!(test.get(&room_state_path).status, Status::Forbidden);
    }

    #[test]
    fn all_the_events_are_retrieved() {
        let test = Test::new();
        let alice = test.create_user();

        let room_options = r##"{
            "initial_state": [{
                "state_key": "",
                "type": "m.room.aliases",
                "content": { "aliases": ["#alias_1:ruma.test"] }
            }, {
                "state_key": "",
                "type": "m.room.topic",
                "content": { "topic": "Test Topic" }
            }, {
                "state_key": "",
                "type": "m.room.name",
                "content": { "name": "Test Name" }
            }, {
                "state_key": "",
                "type": "m.room.canonical_alias",
                "content": { "alias": "#canonical_alias:ruma.test" }
            }]
        }"##;

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let room_state_path = format!(
            "/_matrix/client/r0/rooms/{}/state?access_token={}",
            room_id,
            alice.token
        );

        let response = test.get(&room_state_path);
        assert_eq!(response.status, Status::Ok);

        let events = response.json().as_array().unwrap();
        assert!(events.len() > 0);

        for e in events.iter() {
            match e.get("type").unwrap().as_str().unwrap() {
                "m.room.aliases" => {
                    assert!(
                        e.pointer("/content/aliases").unwrap()
                        .as_array().unwrap()
                        .contains(&Value::String("#alias_1:ruma.test".to_string()))
                    );
                },
                "m.room.canonical_alias" => {
                    assert_eq!(
                        e.pointer("/content/alias").unwrap().as_str().unwrap(),
                        "#canonical_alias:ruma.test"
                    );
                },
                "m.room.create" => {
                    assert_eq!(
                        e.pointer("/content/creator").unwrap().as_str().unwrap(),
                        alice.id
                    );

                    assert_eq!(
                        e.pointer("/content/federate").unwrap().as_bool().unwrap(),
                        true
                    );
                },
                "m.room.history_visibility" => {
                    assert_eq!(
                        e.pointer("/content/history_visibility").unwrap().as_str().unwrap(),
                        "shared"
                    );
                },
                "m.room.join_rules" => {
                    assert_eq!(
                        e.pointer("/content/join_rule").unwrap().as_str().unwrap(),
                        "invite"
                    );
                },
                "m.room.member" => {
                    assert_eq!(
                        e.pointer("/content/membership").unwrap().as_str().unwrap(),
                        "join"
                    );

                    assert_eq!(
                        e.get("sender").unwrap().as_str().unwrap(),
                        format!("{}", alice.id)
                    );
                },
                "m.room.name" => {
                    assert_eq!(
                        e.pointer("/content/name").unwrap().as_str().unwrap(),
                        "Test Name"
                    );
                },
                "m.room.power_levels" => {
                    let ban = e.pointer("/content/ban").unwrap().as_u64().unwrap();
                    let invite = e.pointer("/content/invite").unwrap().as_u64().unwrap();
                    let users = e.pointer("/content/users").unwrap().as_object().unwrap();
                    let creator = e.get("sender").unwrap().as_str().unwrap();

                    // Admin rights.
                    assert_eq!(users.get(&alice.id).unwrap(), &Value::from(100));
                    assert_eq!(creator, alice.id);

                    // The default values.
                    assert_eq!(ban, 50);
                    assert_eq!(invite, 50);
                },
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Test Topic"
                    );
                },
                _ =>  assert!(false)
            }
        }
    }

    #[test]
    fn only_the_latest_events_are_retrieved() {
        let test = Test::new();
        let alice = test.create_user();
        let room_options = r#"{"room_alias_name":"alias_1"}"#;
        let room_id = test.create_room_with_params(&alice.token, &room_options);

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/alias_2?access_token={}",
            alice.token
        );

        let put_room_alias_body = format!(r#"{{"room_id": "{}"}}"#, room_id);

        assert_eq!(
            test.put(&put_room_alias_path, &put_room_alias_body).status,
            Status::Ok
        );

        let room_state_path = format!(
            "/_matrix/client/r0/rooms/{}/state?access_token={}",
            room_id,
            alice.token
        );

        let response = test.get(&room_state_path);
        assert_eq!(response.status, Status::Ok);

        let events = response.json().as_array().unwrap();
        assert!(events.len() > 0);

        for e in events.iter() {
            match e.get("type").unwrap().as_str().unwrap() {
                "m.room.aliases" => {
                    let aliases = e.pointer("/content/aliases").unwrap().as_array().unwrap();

                    assert!(aliases.contains(&Value::String("#alias_1:ruma.test".to_string())));
                    assert!(aliases.contains(&Value::String("#alias_2:ruma.test".to_string())));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn previous_state_for_users_that_left() {
        let test = Test::new();
        let alice = test.create_user();
        let bob = test.create_user();

        let room_options = format!(r#"{{
            "invite": ["{}"],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.topic",
                "content": {{ "topic": "Topic for Bob" }}
            }}]
        }}"#, bob.id);

        let room_id = test.create_room_with_params(&alice.token, &room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);

        let bob_room_state_path = format!(
            "/_matrix/client/r0/rooms/{}/state?access_token={}",
            room_id,
            bob.token
        );

        let response = test.get(&bob_room_state_path);
        assert_eq!(response.status, Status::Ok);

        let events = response.json().as_array().unwrap();
        assert!(events.len() > 0);

        for e in events.iter() {
            match e.get("type").unwrap().as_str().unwrap() {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Topic for Bob"
                    );
                }
                _ => {}
            }
        }

        assert_eq!(test.leave_room(&bob.token, &room_id).status, Status::Ok);

        // Alice updates the topic.
        let event_content = r#"{"topic": "Topic for Alice"}"#;
        let response = test.send_state_event(&alice.token, &room_id, "m.room.topic", &event_content);
        assert_eq!(response.status, Status::Ok);

        // Bob can't see the changes.
        let response = test.get(&bob_room_state_path);
        assert_eq!(response.status, Status::Ok);

        let events = response.json().as_array().unwrap();
        assert!(events.len() > 0);

        for e in events.iter() {
            match e.get("type").unwrap().as_str().unwrap() {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Topic for Bob"
                    );
                }
                _ => {}
            }
        }

        // Alice can see them...
        let alice_room_state_path = format!(
            "/_matrix/client/r0/rooms/{}/state?access_token={}",
            room_id,
            alice.token
        );

        let response = test.get(&alice_room_state_path);
        assert_eq!(response.status, Status::Ok);

        let events = response.json().as_array().unwrap();
        assert!(events.len() > 0);

        for e in events.iter() {
            match e.get("type").unwrap().as_str().unwrap() {
                "m.room.topic" => {
                    assert_eq!(
                        e.pointer("/content/topic").unwrap().as_str().unwrap(),
                        "Topic for Alice"
                    );
                }
                _ => {}
            }
        }
    }
}
