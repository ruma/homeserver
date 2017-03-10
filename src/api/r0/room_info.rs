//! Endpoint for retrieving the state of a room.

use std::convert::TryInto;

use iron::status::Status;
use iron::{Chain, Handler, IronResult, Request, Response};
use router::Router;
use ruma_events::EventType;
use ruma_events::collections::all::StateEvent;
use ruma_events::room::aliases::AliasesEventContent;
use ruma_events::room::avatar::AvatarEventContent;
use ruma_events::room::canonical_alias::CanonicalAliasEventContent;
use ruma_events::room::create::CreateEventContent;
use ruma_events::room::guest_access::GuestAccessEventContent;
use ruma_events::room::history_visibility::HistoryVisibilityEventContent;
use ruma_events::room::join_rules::JoinRulesEventContent;
use ruma_events::room::member::MemberEventContent;
use ruma_events::room::name::NameEventContent;
use ruma_events::room::power_levels::PowerLevelsEventContent;
use ruma_events::room::third_party_invite::ThirdPartyInviteEventContent;
use ruma_events::room::topic::TopicEventContent;
use serde_json::from_str;

use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, EventTypeParam, MiddlewareChain, RoomIdParam};
use models::event::Event;
use models::room::Room;
use models::room_membership::RoomMembership;
use models::user::User;
use modifier::SerializableResponse;

/// Deserialize event's content with the given `EventType` and send it as the response.
macro_rules! send_content {
    ($ty:ty, $content:ident) => {
        {
            let content = from_str::<$ty>($content).map_err(ApiError::from)?;
            Ok(Response::with((Status::Ok, SerializableResponse(content))))
        }
    }
}

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

        let membership = match RoomMembership::find(&connection, &room.id, &user.id)? {
            Some(membership) => membership,
            None => Err(ApiError::unauthorized("The user is not a member of the room".to_string()))?
        };

        let state_events: Vec<StateEvent> = match membership.membership.as_ref() {
            "join" => {
                Event::get_room_full_state(&connection, &room_id)?.iter()
                    .cloned()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<StateEvent>, ApiError>>()?
            },
            "ban" | "leave" => {
                let last_event = Event::find(&connection, &membership.event_id)?
                    .expect("A room membership should be associated with an event");

                Event::get_room_state_events_until(&connection, &room_id, &last_event)?.iter()
                    .cloned()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<StateEvent>, ApiError>>()?
            },
            _ => Err(ApiError::unauthorized("The user is not a member of the room".to_string()))?
        };

        Ok(Response::with((Status::Ok, SerializableResponse(state_events))))
    }
}

/// The `/rooms/:room_id/state/:event_type` and `/rooms/:room_id/state/:event_type/:state_key` endpoints.
pub struct GetStateEvent;

middleware_chain!(GetStateEvent, [RoomIdParam, EventTypeParam, AccessTokenAuth]);

impl Handler for GetStateEvent {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a RoomId").clone();

        let event_type = request.extensions.get::<EventTypeParam>()
            .expect("EventTypeParam should ensure an EventType").clone();

        let state_key = params.find("state_key").unwrap_or("");

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;

        let room = match Room::find(&connection, &room_id)? {
            Some(room) => room,
            None => {
                Err(ApiError::unauthorized("The room was not found on this server".to_string()))?
            }
        };

        let membership = match RoomMembership::find(&connection, &room.id, &user.id)? {
            Some(membership) => membership,
            None => Err(ApiError::unauthorized("The user is not a member of the room".to_string()))?
        };

        let state_event = match membership.membership.as_ref() {
            "join" => {
                Event::get_room_full_state(&connection, &room.id)?.iter()
                    .filter(|e| {
                        e.event_type == event_type.to_string() &&
                        e.state_key.clone().unwrap_or("".to_string()) == state_key
                    })
                    .next()
                    .cloned()
            },
            "ban" | "leave" => {
                let last_event = Event::find(&connection, &membership.event_id)?
                    .expect("A room membership should be associated with an event");

                Event::get_room_state_events_until(&connection, &room_id, &last_event)?.iter()
                    .filter(|e| {
                        e.event_type == event_type.to_string() &&
                        e.state_key.clone().unwrap_or("".to_string()) == state_key
                    })
                    .next()
                    .cloned()
            },
            _ => Err(ApiError::unauthorized("The user is not a member of the room".to_string()))?
        };

        if state_event.is_none() {
            Err(ApiError::not_found("The requested state event was not found".to_string()))?
        }

        let content = &state_event.unwrap().content.clone();

        match event_type {
            EventType::RoomAliases => send_content!(AliasesEventContent, content),
            EventType::RoomAvatar => send_content!(AvatarEventContent, content),
            EventType::RoomCanonicalAlias => send_content!(CanonicalAliasEventContent, content),
            EventType::RoomCreate => send_content!(CreateEventContent, content),
            EventType::RoomGuestAccess => send_content!(GuestAccessEventContent, content),
            EventType::RoomHistoryVisibility => send_content!(HistoryVisibilityEventContent, content),
            EventType::RoomJoinRules => send_content!(JoinRulesEventContent, content),
            EventType::RoomMember => send_content!(MemberEventContent, content),
            EventType::RoomName => send_content!(NameEventContent, content),
            EventType::RoomPowerLevels => send_content!(PowerLevelsEventContent, content),
            EventType::RoomThirdPartyInvite => send_content!(ThirdPartyInviteEventContent, content),
            EventType::RoomTopic => send_content!(TopicEventContent, content),
            _ => Err(ApiError::bad_event("Unsupported state event type".to_string()))?,
        }
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
        let response = test.send_state_event(&alice.token, &room_id, "m.room.topic", &event_content, None);
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

    #[test]
    fn retrieve_state_event_by_type() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures("{}");

        let response = test.get_state_event(&alice.token, &room_id, "m.room.create", None);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("creator").unwrap().as_str().unwrap(), &alice.id);

        let topic_content = r#"{"topic": "Initial Topic"}"#;
        let response = test.send_state_event(&alice.token, &room_id, "m.room.topic", topic_content, None);
        assert_eq!(response.status, Status::Ok);

        let response = test.get_state_event(&alice.token, &room_id, "m.room.topic", None);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("topic").unwrap().as_str().unwrap(), "Initial Topic");

        // Change the topic again to ensure we only get the latest version of the event.
        let topic_content = r#"{"topic": "Updated Topic"}"#;
        let response = test.send_state_event(&alice.token, &room_id, "m.room.topic", topic_content, None);
        assert_eq!(response.status, Status::Ok);

        let response = test.get_state_event(&alice.token, &room_id, "m.room.topic", None);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("topic").unwrap().as_str().unwrap(), "Updated Topic");
    }

    #[test]
    fn retrieve_state_event_by_type_and_key() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures("{}");

        let third_party_invite_content = r#"{
            "display_name": "Alice",
            "key_validity_url": "https://magic.forest/verifykey",
            "public_key": "abc123"
        }"#;
        let response = test.send_state_event(
            &alice.token,
            &room_id,
            "m.room.third_party_invite",
            third_party_invite_content,
            Some("pc89")
        );
        assert_eq!(response.status, Status::Ok);

        let response = test.get_state_event(&alice.token, &room_id, "m.room.third_party_invite", Some("pc89"));
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("public_key").unwrap().as_str().unwrap(), "abc123");

        let response = test.get_state_event(&alice.token, &room_id, "m.room.third_party_invite", Some("pc100"));
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn retrieve_state_event_from_left_room() {
        let test = Test::new();
        let bob = test.create_user();
        let room_options = format!(r#"{{
            "invite": ["{}"],
            "initial_state": [{{
                "state_key": "",
                "type": "m.room.name",
                "content": {{ "name": "Initial Name" }}
            }}]
        }}"#, bob.id);
        let (alice, room_id) = test.initial_fixtures(&room_options);

        assert_eq!(test.join_room(&bob.token, &room_id).status, Status::Ok);
        assert_eq!(test.leave_room(&bob.token, &room_id).status, Status::Ok);

        let name_content = r#"{"name": "Updated Name"}"#;
        let response = test.send_state_event(&alice.token, &room_id, "m.room.name", name_content, None);
        assert_eq!(response.status, Status::Ok);

        let topic_content = r#"{"topic": "Initial Topic"}"#;
        let response = test.send_state_event(&alice.token, &room_id, "m.room.topic", topic_content, None);
        assert_eq!(response.status, Status::Ok);

        let response = test.get_state_event(&bob.token, &room_id, "m.room.name", None);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.json().get("name").unwrap().as_str().unwrap(), "Initial Name");

        let response = test.get_state_event(&bob.token, &room_id, "m.room.topic", None);
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn non_existent_state_event_type() {
        let test = Test::new();
        let (alice, room_id) = test.initial_fixtures("{}");

        let response = test.get_state_event(&alice.token, &room_id, "m.room.create", Some("unknown"));
        assert_eq!(response.status, Status::NotFound);

        let response = test.get_state_event(&alice.token, &room_id, "m.room.unknown", None);
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn retrieve_state_event_non_member() {
        let test = Test::new();
        let bob = test.create_user();
        let carl = test.create_user();
        let room_options = format!(r#"{{"invite": ["{}"]}}"#, carl.id);
        let (_, room_id) = test.initial_fixtures(&room_options);

        // Neither Bob nor Carl can retrieve state events.
        let response = test.get_state_event(&bob.token, &room_id, "m.room.create", None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The user is not a member of the room"
        );

        let response = test.get_state_event(&carl.token, &room_id, "m.room.create", None);
        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "The user is not a member of the room"
        );
    }
}
