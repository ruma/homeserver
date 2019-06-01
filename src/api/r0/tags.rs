//! Endpoints for tags.
use std::collections::HashMap;

use bodyparser;
use iron::status::Status;
use iron::{Chain, Handler, IronResult, Plugin, Request, Response};
use ruma_events::tag::TagInfo;
use serde_json::Value;

use crate::db::DB;
use crate::error::ApiError;
use crate::middleware::{
    AccessTokenAuth, JsonRequest, MiddlewareChain, RoomIdParam, TagParam, UserIdParam,
};
use crate::models::tags::RoomTag;
use crate::models::user::User;
use crate::modifier::{EmptyResponse, SerializableResponse};

/// The GET `/user/:user_id/rooms/:room_id/tags` endpoint.
pub struct GetTags;

middleware_chain!(GetTags, [UserIdParam, RoomIdParam, AccessTokenAuth]);

#[derive(Debug, Serialize)]
pub struct TagsResponse {
    tags: HashMap<String, TagInfo>,
}

impl Handler for GetTags {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let user_id = request
            .extensions
            .get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId")
            .clone();
        let room_id = request
            .extensions
            .get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a RoomId")
            .clone();
        let user = request
            .extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            Err(ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string(),
            ))?;
        }

        let connection = DB::from_request(request)?;

        let tags = RoomTag::find(&connection, user_id, room_id)?;

        let response = TagsResponse { tags };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The PUT `/user/:user_id/rooms/:room_id/tags/:tag` endpoint.
pub struct PutTag;

middleware_chain!(
    PutTag,
    [
        UserIdParam,
        RoomIdParam,
        TagParam,
        JsonRequest,
        AccessTokenAuth
    ]
);

impl Handler for PutTag {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let user_id = request
            .extensions
            .get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId")
            .clone();
        let room_id = request
            .extensions
            .get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a RoomId")
            .clone();
        let tag = request
            .extensions
            .get::<TagParam>()
            .expect("TagParam should ensure a tag")
            .clone();
        let user = request
            .extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            Err(ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string(),
            ))?;
        }

        let content = match request.get::<bodyparser::Struct<Value>>() {
            Ok(Some(content)) => content.to_string(),
            Ok(None) => "".to_string(),
            Err(_) => Err(ApiError::bad_json(None))?,
        };

        let connection = DB::from_request(request)?;

        RoomTag::upsert(&connection, user_id, room_id, tag, content)?;

        Ok(Response::with(EmptyResponse(Status::Ok)))
    }
}

/// The DELETE `/user/:user_id/rooms/:room_id/tags/:tag` endpoint.
pub struct DeleteTag;

middleware_chain!(
    DeleteTag,
    [UserIdParam, RoomIdParam, TagParam, AccessTokenAuth]
);

impl Handler for DeleteTag {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let user_id = request
            .extensions
            .get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId")
            .clone();
        let room_id = request
            .extensions
            .get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a RoomId")
            .clone();
        let user = request
            .extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();
        let tag = request
            .extensions
            .get::<TagParam>()
            .expect("TagParam should ensure a tag")
            .clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            Err(ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string(),
            ))?;
        }

        let connection = DB::from_request(request)?;

        RoomTag::delete(&connection, user_id, room_id, tag)?;

        Ok(Response::with(EmptyResponse(Status::Ok)))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::Test;
    use iron::status::Status;

    #[test]
    fn put_tag() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = test.create_public_room(&carl.token);

        test.create_tag(
            &carl.token,
            &room_id,
            &carl.id,
            "work",
            r#"{"order":"test"}"#,
        );

        let get_tags_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.get(&get_tags_path);
        assert_eq!(response.status, Status::Ok);
        let chunk = response.json().get("tags").unwrap();
        assert!(chunk.is_object());
        let chunk = chunk.as_object().unwrap();
        assert_eq!(chunk.len(), 1);
        let content = chunk.get("work").unwrap();
        assert_eq!(content.to_string(), r#"{"order":"test"}"#);
    }

    #[test]
    fn get_tags_forbidden() {
        let test = Test::new();
        let carl = test.create_user();
        let alice = test.create_user();
        let room_id = test.create_public_room(&carl.token);

        let get_tags_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags?access_token={}",
            carl.id, room_id, alice.token
        );

        let response = test.get(&get_tags_path);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn put_tag_forbidden() {
        let test = Test::new();
        let carl = test.create_user();
        let alice = test.create_user();

        let room_id = test.create_public_room(&carl.token);
        let put_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/{}?access_token={}",
            carl.id, room_id, "work", alice.token
        );

        let response = test.put(&put_tag_path, r#"{}"#);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn delete_tag_forbidden() {
        let test = Test::new();
        let carl = test.create_user();
        let alice = test.create_user();

        let room_id = test.create_public_room(&carl.token);

        test.create_tag(
            &carl.token,
            &room_id,
            carl.id.as_str(),
            "delete",
            r#"{"order":"test"}"#,
        );

        let delete_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/delete?access_token={}",
            carl.id, room_id, alice.token
        );

        let response = test.delete(&delete_tag_path);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn double_put_should_update_tag() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = test.create_public_room(&carl.token);

        test.create_tag(
            &carl.token,
            &room_id,
            &carl.id,
            "test",
            r#"{"order":"test"}"#,
        );

        test.create_tag(
            &carl.token,
            &room_id,
            &carl.id,
            "test",
            r#"{"order":"test2"}"#,
        );

        let get_tags_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.get(&get_tags_path);
        let chunk = response.json().get("tags").unwrap();
        let chunk = chunk.as_object().unwrap();
        let content = chunk.get("test").unwrap();
        assert_eq!(content.to_string(), r#"{"order":"test2"}"#);
    }

    #[test]
    fn delete_tag() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = test.create_public_room(&carl.token);

        test.create_tag(
            &carl.token,
            &room_id,
            &carl.id,
            "delete",
            r#"{"order":"test"}"#,
        );

        let delete_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/delete?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.delete(&delete_tag_path);
        assert_eq!(response.status, Status::Ok);

        let get_tags_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.get(&get_tags_path);
        assert_eq!(response.status, Status::Ok);
        let chunk = response.json().get("tags").unwrap();
        let chunk = chunk.as_object().unwrap();
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn put_tag_invalid_room() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = "!n8f893n9:ruma.test";

        let put_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/{}?access_token={}",
            carl.id, room_id, "work", carl.token
        );

        let response = test.get(&put_tag_path);
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn get_tags_invalid_room() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = "!n8f893n9:ruma.test";

        let get_tags_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.get(&get_tags_path);
        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn delete_tags_invalid_room_and_tag() {
        let test = Test::new();
        let carl = test.create_user();
        let room_id = test.create_public_room(&carl.token);

        let delete_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/test?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.delete(&delete_tag_path);
        assert_eq!(response.status, Status::NotFound);

        let room_id = "!n8f893n9:ruma.test";

        let delete_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/test?access_token={}",
            carl.id, room_id, carl.token
        );

        let response = test.delete(&delete_tag_path);
        assert_eq!(response.status, Status::NotFound);
    }
}
