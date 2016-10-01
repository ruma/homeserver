//! Endpoints for room members.

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use ruma_events::room::member::MemberEvent;

use db::DB;
use room_membership::RoomMembership;
use middleware::{AccessTokenAuth, RoomIdParam};
use modifier::SerializableResponse;
use user::User;

/// The `/rooms/:room_id/members` endpoint.
pub struct Members;

#[derive(Debug, Serialize)]
struct MembersResponse {
    chunk: Vec<MemberEvent>,
}

impl Members {
    /// Create a `Members` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Members);

        chain.link_before(AccessTokenAuth);
        chain.link_before(RoomIdParam);

        chain
    }
}

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
        let access_token = test.create_access_token();
        let room_id = test.create_public_room(&access_token);

        let room_join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            access_token
        );
        test.post(&room_join_path, r"{}");

        let room_members_path = format!(
            "/_matrix/client/r0/rooms/{}/members?access_token={}",
            room_id,
            access_token
        );

        let response = test.get(&room_members_path);
        assert_eq!(response.status, Status::Ok);
        let chunk = response.json().find("chunk").unwrap();
        assert!(chunk.is_array());
        let chunk = chunk.as_array().unwrap();
        assert_eq!(chunk.len(), 1);
    }
}