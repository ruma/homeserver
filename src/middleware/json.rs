use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Plugin, Request};
use iron::typemap::Key;
use rustc_serialize;

use error::APIError;

pub struct Json;

impl Key for Json {
    type Value = rustc_serialize::json::Json;
}

impl BeforeMiddleware for Json {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        match request.get::<bodyparser::Json>() {
            Ok(Some(json)) => {
                request.extensions.insert::<Json>(json);

                Ok(())
            },
            Ok(None) => {
                let error = APIError::not_json();

                Err(IronError::new(error.clone(), error))
            },
            Err(err) => {
                let error = APIError::bad_json(&err);

                Err(IronError::new(error.clone(), error))
           }
        }
    }
}

