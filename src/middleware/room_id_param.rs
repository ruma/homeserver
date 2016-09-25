
use std::convert::TryFrom;

use iron::{BeforeMiddleware, IronResult, Request};
use iron::typemap::Key;
use router::Router;
use ruma_identifiers::RoomId;

use error::{ApiError, MapApiError};

/// Handles convert `room_id` param to `RoomId`.
pub struct RoomIdParam;

impl Key for RoomIdParam {
    type Value = RoomId;
}

impl BeforeMiddleware for RoomIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();
        let room_id = match params.find("room_id") {
            Some(room_id) => RoomId::try_from(room_id).map_api_err(|_| {
                ApiError::not_found(Some(&format!("No room found with ID {}", room_id)))
            }),
            None => {
                Err(ApiError::missing_param("room_id"))
            }
        }?;
        request.extensions.insert::<RoomIdParam>(room_id);
        Ok(())
    }
}