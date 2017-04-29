//! Endpoints for managing room aliases.

use bodyparser;
use iron::{Chain, Handler, IronResult, Plugin, Request, Response};
use iron::status::Status;
use ruma_identifiers::RoomId;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain, RoomAliasIdParam};
use models::room_alias::{RoomAlias, NewRoomAlias};
use models::user::User;
use modifier::{SerializableResponse, EmptyResponse};

/// The GET `/directory/room/:room_alias` endpoint.
pub struct GetRoomAlias;

#[derive(Debug, Serialize)]
struct GetRoomAliasResponse {
    /// The room ID associated with the room alias.
    room_id: RoomId,
    /// A list of servers that are aware of this room ID.
    servers: Vec<String>,
}

middleware_chain!(GetRoomAlias, [RoomAliasIdParam]);

impl Handler for GetRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let room_alias_id = request.extensions.get::<RoomAliasIdParam>()
            .expect("RoomAliasIdParam should ensure a RoomAliasId").clone();

        let connection = DB::from_request(request)?;

        let room_alias = RoomAlias::find_by_alias(&connection, &room_alias_id)?;

        let response = GetRoomAliasResponse {
            room_id: room_alias.room_id,
            servers: room_alias.servers,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The DELETE `/directory/room/:room_alias` endpoint.
pub struct DeleteRoomAlias;

middleware_chain!(DeleteRoomAlias, [RoomAliasIdParam, AccessTokenAuth]);

impl Handler for DeleteRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let room_alias_id = request.extensions.get::<RoomAliasIdParam>()
            .expect("RoomAliasIdParam should ensure a RoomAliasId").clone();

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;

        let affected_rows = RoomAlias::delete(&connection, &room_alias_id, &user.id)?;

        if affected_rows > 0 {
            Ok(Response::with(EmptyResponse(Status::Ok)))
        } else {
            Err(ApiError::not_found(
                "Provided room alias did not exist or you do not have access to delete it.".to_string()
            ))?
        }
    }
}

/// The PUT `/directory/room/:room_alias` endpoint.
pub struct PutRoomAlias;

#[derive(Clone, Debug, Deserialize)]
struct PutRoomAliasRequest {
    /// The room ID for which the alias will be set.
    pub room_id: RoomId,
}

middleware_chain!(PutRoomAlias, [JsonRequest, RoomAliasIdParam, AccessTokenAuth]);

impl Handler for PutRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let config = Config::from_request(request)?;

        let room_alias_id = request.extensions.get::<RoomAliasIdParam>()
            .expect("RoomAliasIdParam should ensure a RoomAliasId").clone();

        let room_id = match request.get::<bodyparser::Struct<PutRoomAliasRequest>>() {
            Ok(Some(req)) => req.room_id,
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let connection = DB::from_request(request)?;

        let new_room_alias = NewRoomAlias {
            alias: room_alias_id,
            room_id: room_id,
            user_id: user.id,
            servers: vec![config.domain.to_string()],
        };

        RoomAlias::create(&connection, &config.domain.to_string(), &new_room_alias)?;

        Ok(Response::with(Status::Ok))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn get_room_alias() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);
        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().get("room_id").unwrap().as_str().unwrap();

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().get("room_id").unwrap().as_str().unwrap(), room_id);
        assert!(response.json().get("servers").unwrap().is_array());
    }

    #[test]
    fn get_unknown_room_alias() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);
        let _ = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        let response = test.get("/_matrix/client/r0/directory/room/no_room");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "M_NOT_FOUND"
        );
    }

    #[test]
    fn delete_room_alias() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!(
            "/_matrix/client/r0/createRoom?access_token={}",
            user.token
        );

        test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        let delete_room_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}",
            user.token
        );

        let delete_response = test.delete(&delete_room_path);

        assert_eq!(delete_response.status, Status::Ok);

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn delete_room_alias_from_different_user() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!(
            "/_matrix/client/r0/createRoom?access_token={}",
            user.token
        );

        test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        let henry = test.create_user();

        let delete_room_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}",
            henry.token
        );

        let response = test.delete(&delete_room_path);

        assert_eq!(response.status, Status::NotFound);
    }

    #[test]
    fn put_room_alias() {
        let test = Test::new();
        let (carl, room_id) = test.initial_fixtures(r#"{"visibility": "public"}"#);

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}", carl.token
        );
        let put_room_alias_body = format!(r#"{{"room_id": "{}"}}"#, room_id);
        let response = test.put(&put_room_alias_path, &put_room_alias_body);

        assert_eq!(response.status, Status::Ok);

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().get("room_id").unwrap().as_str().unwrap(), room_id);
        assert!(response.json().get("servers").unwrap().is_array());
    }

    #[test]
    fn put_room_alias_with_no_room() {
        let test = Test::new();
        let user = test.create_user();

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}", user.token
        );
        let put_room_alias_body = r#"{"room_id": "!nonexistent:ruma.test"}"#;
        let response = test.put(&put_room_alias_path, &put_room_alias_body);

        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn put_existing_room_alias() {
        let test = Test::new();
        let user = test.create_user();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       user.token);
        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().get("room_id").unwrap().as_str().unwrap();

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}", user.token
        );
        let put_room_alias_body = format!(r#"{{"room_id": "{}"}}"#, room_id);
        let response = test.put(&put_room_alias_path, &put_room_alias_body);

        assert_eq!(response.status, Status::Conflict);
        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "IO_RUMA_ALIAS_TAKEN"
        );
    }
}
