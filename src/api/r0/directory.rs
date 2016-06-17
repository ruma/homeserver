//! Endpoints for managing room aliases.

use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;
use router::Router;

use db::DB;
use error::APIError;
use modifier::SerializableResponse;
use room_alias::RoomAlias;

#[derive(Debug, Serialize)]
struct GetDirectoryRoomResponse {
    room_id: String,
    servers: Vec<String>,
}

/// The /directory/room/{roomAlias} endpoint when using the GET method.
pub struct GetDirectoryRoom;

/// The /directory/room/{roomAlias} endpoint for the DELETE method.
pub struct DeleteDirectoryRoom;

impl GetDirectoryRoom {
    /// Create a `DirectoryRoom`.
    pub fn chain() -> Chain {
        Chain::new(GetDirectoryRoom)
    }
}

impl DeleteDirectoryRoom {
    /// Delete a `DirectoryRoom` mapping.
    pub fn chain() -> Chain {
        Chain::new(DeleteDirectoryRoom)
    }
}

impl Handler for GetDirectoryRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_alias_name = params.find("room_alias").ok_or(APIError::not_found())?;

        let connection = DB::from_request(request)?;

        let room_alias = RoomAlias::find_by_alias(&connection, room_alias_name)?;

        let response = GetDirectoryRoomResponse {
            room_id: room_alias.room_id,
            servers: room_alias.servers,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

impl Handler for DeleteDirectoryRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();

        let room_alias_name = params.find("room_alias").ok_or(APIError::not_found())?;

        let connection = DB::from_request(request)?;

        RoomAlias::delete(&connection, room_alias_name)?;

        // Respond with an empty JSON object
        Ok(Response::with((Status::Ok, "{}")))
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
        let room_id = response.json().find("room_id").unwrap().as_string();

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().find("room_id").unwrap().as_string(), room_id);
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
            response.json().find("errcode").unwrap().as_string().unwrap(),
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
        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);
        let room_id = response.json().find("room_id").unwrap().as_string();

        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.json().find("room_id").unwrap().as_string(), room_id);
        assert!(response.json().find("servers").unwrap().is_array());

        // Delete the room
        test.delete("/_matrix/client/r0/directory/room/my_room", "");

        assert_eq!(response.json().find("room_id").unwrap().as_string(), room_id);
        assert!(response.json().find("servers").unwrap().is_array());

        // Make sure the room no longer exists
        let response = test.get("/_matrix/client/r0/directory/room/my_room");

        assert_eq!(response.status, Status::NotFound);
        assert_eq!(
            response.json().find("errcode").unwrap().as_string().unwrap(),
            "M_NOT_FOUND"
        );
    }
}
