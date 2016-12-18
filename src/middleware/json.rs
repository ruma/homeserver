use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Plugin, Request};
use iron::headers::ContentType;
use iron::mime::{Mime, SubLevel, TopLevel};
use iron::typemap::Key;
use serde_json::Value;

use error::ApiError;

/// Ensures that requests contain valid JSON and stores the parsed JSON in the Iron request.
pub struct JsonRequest;

impl Key for JsonRequest {
    type Value = Value;
}

impl BeforeMiddleware for JsonRequest {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        if request.headers.get::<ContentType>().and_then(|content_type| match **content_type {
            Mime(TopLevel::Application, SubLevel::Json, _) => Some(()),
            _ => None,
        }).is_none() {
            return Err(IronError::from(ApiError::wrong_content_type(None)));
        }

        match request.get::<bodyparser::Json>() {
            Ok(Some(_)) => Ok(()),
            Ok(_) | Err(_) => {
                Err(IronError::from(ApiError::not_json(None)))
            },
        }
    }
}
