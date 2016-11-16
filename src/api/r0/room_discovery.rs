//! Endpoints for room discovery.

use modifier::SerializableResponse;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};

#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomsResponse {
    pub chunk: Option<Vec<PublicRoomsChunk>>, // A paginated chunk of public rooms.
    pub end: Option<String>, // A pagination token for the response.
    pub start: Option<String>, // A pagination token for the response.
}

/// A helper struct for showing a single room
#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomsChunk {
    pub aliases: Option<Vec<String>>, // Aliases of the room. May be empty.
    pub avatar_url: Option<String>, // The URL for the room's avatar, if one is set.
    pub guest_can_join: Option<bool>, // Whether guest users may join the room and participate in it. If they can, they will be subject to ordinary power level rules like any other user.
    pub name: Option<String>, // The name of the room, if any. May be null.
    pub num_joined_members: Option<usize>, // The number of members joined to the room.
    pub room_id: Option<String>, // The ID of the room.
    pub topic: Option<String>, // The topic of the room, if any. May be null.
    pub world_readable: Option<bool>, // Whether the room may be viewed by guest users without joining.
}

/// The /publicRooms endpoint.
pub struct PublicRooms;

impl PublicRooms {
    /// Creates a new instance of the PublicRooms handler
    pub fn new() -> Self {
        PublicRooms {}
    }
}

impl Handler for PublicRooms {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let response = PublicRoomsResponse { chunk: None, end: None, start: None };
        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}
