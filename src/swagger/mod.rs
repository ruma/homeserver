//! Data for [Swagger UI](https://github.com/swagger-api/swagger-ui).

use iron::{Chain, Handler, IronResult, Request, Response, status};
use iron::headers::ContentType;
use iron::modifiers::Header;

use middleware::{MiddlewareChain, ResponseHeaders};

/// Mounts the Swagger endpoint onto the given `Mount`.
pub struct Swagger;

impl Handler for Swagger {
    fn handle(&self, _request: &mut Request) -> IronResult<Response> {
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
