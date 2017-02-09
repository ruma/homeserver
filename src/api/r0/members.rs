//! Endpoints for room members.

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use ruma_events::room::member::MemberEvent;

use db::DB;
use middleware::{AccessTokenAuth, MiddlewareChain, RoomIdParam};
use models::room_membership::RoomMembership;
use models::user::User;
use modifier::SerializableResponse;

/// The `/rooms/:room_id/members` endpoint.
pub struct Members;

#[derive(Debug, Serialize)]
struct MembersResponse {
    chunk: Vec<MemberEvent>,
}

middleware_chain!(Members, [RoomIdParam, AccessTokenAuth]);

impl Handler for Members {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        request.extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user");

        let connection = DB::from_request(request)?;

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("Should have been required by RoomIdParam.")
            .clone();

        let events = RoomMembership::get_events_by_room(&connection, room_id)?;

        let response = MembersResponse { chunk: events };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn room_members() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            carl.token
        );
        test.post(&room_join_path, r"{}");

        let room_members_path = format!(
            "/_matrix/client/r0/rooms/{}/members?access_token={}",
            room_id,
            carl.token
        );

        let response = test.get(&room_members_path);
        assert_eq!(response.status, Status::Ok);
        let chunk = response.json().get("chunk").unwrap();
        assert!(chunk.is_array());
        let chunk = chunk.as_array().unwrap();
        assert_eq!(chunk.len(), 1);
    }
}
