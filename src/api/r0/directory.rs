//! Endpoints for managing room aliases.

use std::convert::TryFrom;

use bodyparser;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;
use router::Router;
use ruma_identifiers::RoomId;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest};
use modifier::SerializableResponse;
use room_alias::{RoomAlias, NewRoomAlias};

#[derive(Debug, Serialize)]
struct GetRoomAliasResponse {
    room_id: String,
    servers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PutRoomAliasRequest {
    pub room_id: String,
}

/// The GET /directory/room/:room_alias endpoint.
pub struct GetRoomAlias;

/// The DELETE /directory/room/:room_alias endpoint.
pub struct DeleteRoomAlias;

/// The PUT /directory/room/:room_alias endpoint.
pub struct PutRoomAlias;

impl GetRoomAlias {
    /// Create a `GetRoomAlias`.
    pub fn chain() -> Chain {
        Chain::new(GetRoomAlias)
    }
}

impl DeleteRoomAlias {
    /// Create a `DeleteRoomAlias` with necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(DeleteRoomAlias);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl PutRoomAlias {
    /// Create a `PutRoomAlias` with necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(PutRoomAlias);

        chain.link_before(JsonRequest);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for GetRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_alias_name = params.find("room_alias").ok_or(ApiError::not_found(None))?;

        let connection = DB::from_request(request)?;

        let room_alias = RoomAlias::find_by_alias(&connection, room_alias_name)?;

        let response = GetRoomAliasResponse {
            room_id: room_alias.room_id.to_string(),
            servers: room_alias.servers,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

impl Handler for DeleteRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_alias_name = params.find("room_alias").ok_or(ApiError::not_found(None))?;

        let connection = DB::from_request(request)?;

        RoomAlias::delete(&connection, room_alias_name)?;

        Ok(Response::with((Status::Ok, "{}")))
    }
}

impl Handler for PutRoomAlias {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_alias_name = params.find("room_alias").ok_or(ApiError::not_found(None))?;

        let parsed_request = request.get::<bodyparser::Struct<PutRoomAliasRequest>>();
        let room_id = if let Ok(Some(api_request)) = parsed_request {
            RoomId::try_from(&api_request.room_id).map_err(ApiError::from)?
        } else {
            let error = ApiError::bad_json(None);

            return Err(IronError::new(error.clone(), error));
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let new_room_alias = NewRoomAlias {
            alias: room_alias_name.to_string(),
            room_id: room_id,
            servers: vec![config.domain.to_string()],
        };

        RoomAlias::create(&connection, &new_room_alias)?;

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
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);
        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().find("room_id").unwrap().as_str().unwrap();

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().find("room_id").unwrap().as_str().unwrap(), room_id);
        assert!(response.json().find("servers").unwrap().is_array());
    }

    #[test]
    fn get_unknown_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);
        let _ = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        let response = test.get("/_matrix/client/r0/directory/room/no_room");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_NOT_FOUND"
        );
    }

    #[test]
    fn delete_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();

        // Create a room
        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);
        test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        // Delete the room alias
        let delete_room_path = format!("/_matrix/client/r0/directory/room/my_room?access_token={}",
                                       access_token);
        let response = test.delete(&delete_room_path);

        assert_eq!(response.status, Status::Ok);

        // Make sure the room no longer exists
        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_NOT_FOUND"
        );
    }

    #[test]
    fn put_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let room_id = test.create_room(&access_token);

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}", access_token
        );
        let put_room_alias_body = format!(r#"{{"room_id": "{}"}}"#, room_id);
        let response = test.put(&put_room_alias_path, &put_room_alias_body);

        assert_eq!(response.status, Status::Ok);

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().find("room_id").unwrap().as_str().unwrap(), room_id);
        assert!(response.json().find("servers").unwrap().is_array());
    }

    #[test]
    fn put_existing_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?access_token={}",
                                       access_token);
        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().find("room_id").unwrap().as_str().unwrap();

        let put_room_alias_path = format!(
            "/_matrix/client/r0/directory/room/my_room?access_token={}", access_token
        );
        let put_room_alias_body = format!(r#"{{"room_id": "{}"}}"#, room_id);
        let response = test.put(&put_room_alias_path, &put_room_alias_body);

        assert_eq!(response.status, Status::Conflict);
        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "IO_RUMA_ALIAS_TAKEN"
        );
    }
}
