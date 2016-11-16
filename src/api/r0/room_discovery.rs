//! Endpoints for room discovery.

use modifier::SerializableResponse;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};

#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomsResponse {
    /// A paginated chunk of public rooms.
    pub chunk: Option<Vec<PublicRoomsChunk>>,
    /// A pagination token for the response.
    pub end: Option<String>,
    /// A pagination token for the response.
    pub start: Option<String>,
}

/// A helper struct for showing a single room
#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomsChunk {
    /// Aliases of the room. May be empty.
    pub aliases: Option<Vec<String>>,
    /// The URL for the room's avatar, if one is set.
    pub avatar_url: Option<String>,
    /// Whether guest users may join the room and participate in it. If they can, they will be subject to ordinary power level rules like any other user.
    pub guest_can_join: Option<bool>,
    /// The name of the room, if any. May be null.
    pub name: Option<String>,
    /// The number of members joined to the room.
    pub num_joined_members: Option<usize>,
    /// The ID of the room.
    pub room_id: Option<String>,
    /// The topic of the room, if any. May be null.
    pub topic: Option<String>,
    /// Whether the room may be viewed by guest users without joining.
    pub world_readable: Option<bool>,
}

/// The /publicRooms endpoint.
pub struct GetPublicRooms;

impl GetPublicRooms {
    /// Create a `GetPublicRooms` with all necessary middleware.
    pub fn chain() -> Chain {
        Chain::new(GetPublicRooms)
    }
}

impl Handler for GetPublicRooms {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let response = PublicRoomsResponse { chunk: None, end: None, start: None };
        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}
