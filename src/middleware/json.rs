//! Iron middleware to handle verifying the presence of valid JSON in a request.

use bodyparser;
use iron::headers::ContentType;
use iron::mime::{Mime, SubLevel, TopLevel};
use iron::typemap::Key;
use iron::{BeforeMiddleware, IronResult, Plugin, Request};
use serde_json::Value;

use crate::error::ApiError;

/// Ensures that requests contain valid JSON and stores the parsed JSON in the Iron request.
#[derive(Clone, Copy, Debug)]
pub struct JsonRequest;

impl Key for JsonRequest {
    type Value = Value;
}

impl BeforeMiddleware for JsonRequest {
    fn before(&self, request: &mut Request<'_, '_>) -> IronResult<()> {
        if request
            .headers
            .get::<ContentType>()
            .and_then(|content_type| match **content_type {
                Mime(TopLevel::Application, SubLevel::Json, _) => Some(()),
                _ => None,
            })
            .is_none()
        {
            Err(ApiError::wrong_content_type(None))?
        }

        match request.get::<bodyparser::Json>() {
            Ok(Some(_)) => Ok(()),
            Ok(_) | Err(_) => Err(ApiError::not_json(None))?,
        }
    }
}
