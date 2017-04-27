//! Endpoints for content.
use iron::{Chain, Handler, IronError, IronResult, Request, Response};

use error::ApiError;
use middleware::{AccessTokenAuth, MiddlewareChain};

/// The `/preview_url` endpoint.
pub struct PreviewUrl;

middleware_chain!(PreviewUrl, [AccessTokenAuth]);

impl Handler for PreviewUrl {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Err(IronError::from(ApiError::unimplemented(None)))
    }
}
