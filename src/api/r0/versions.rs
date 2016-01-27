use iron::{Handler, IronResult, Request, Response, status};

use modifier::SerializableResponse;

pub struct Versions {
    versions: Vec<&'static str>,
}

impl Versions {
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
