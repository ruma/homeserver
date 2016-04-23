use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Plugin, Request};
use iron::headers::ContentType;
use iron::mime::{Mime, SubLevel, TopLevel};
use iron::typemap::Key;
use serde_json::Value;

use error::APIError;

/// Ensures that requests contain valid JSON and stores the parsed JSON in the Iron request.
pub struct JsonRequest;

impl Key for JsonRequest {
    type Value = Value;
}

impl BeforeMiddleware for JsonRequest {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        match request.headers.get::<ContentType>() {
            Some(content_type) => {
                match **content_type {
                    Mime(TopLevel::Application, SubLevel::Json, _) => {},
                    _ => {
                        let error = APIError::wrong_content_type();

                        return Err(IronError::new(error.clone(), error));
                    }
                }
            },
            None => {
                let error = APIError::wrong_content_type();

                return Err(IronError::new(error.clone(), error));
            },
        }

        match request.get::<bodyparser::Json>() {
            Ok(Some(json)) => {
                request.extensions.insert::<JsonRequest>(json);

                Ok(())
            },
            Ok(_) | Err(_) => {
                let error = APIError::not_json();

                Err(IronError::new(error.clone(), error))
            },
        }
    }
}
