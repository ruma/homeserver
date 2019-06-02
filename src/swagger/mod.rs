//! Data for [Swagger UI](https://github.com/swagger-api/swagger-ui).

use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::{status, Chain, Handler, IronResult, Request, Response};

use crate::middleware::{MiddlewareChain, ResponseHeaders};

/// Mounts the Swagger endpoint onto the given `Mount`.
#[derive(Clone, Copy, Debug)]
pub struct Swagger;

impl Handler for Swagger {
    fn handle(&self, _request: &mut Request<'_, '_>) -> IronResult<Response> {
        let json = include_str!("swagger.json");

        Ok(Response::with((
            status::Ok,
            Header(ContentType::json()),
            json,
        )))
    }
}

impl MiddlewareChain for Swagger {
    /// Create a `Swagger` with all necessary middleware.
    fn chain() -> Chain {
        let mut chain = Chain::new(Swagger);

        chain.link_after(ResponseHeaders);

        chain
    }
}
