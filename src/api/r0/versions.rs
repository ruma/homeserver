//! Endpoints for information about supported versions of the Matrix spec.

use iron::{Handler, IronResult, Request, Response, status};

use modifier::SerializableResponse;

/// The /versions endpoint.
pub struct Versions {
    versions: Vec<&'static str>,
}

impl Versions {
    /// Create a `Versions` offering support for the specified versions of the Matrix spec.
    pub fn new(versions: Vec<&'static str>) -> Versions {
        Versions {
            versions: versions,
        }
    }
}

impl Handler for Versions {
    fn handle(&self, _request: &mut Request) -> IronResult<Response> {
        Ok(Response::with((status::Ok, SerializableResponse(&self.versions))))
    }
}
