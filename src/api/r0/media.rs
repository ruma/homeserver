//! Endpoints for content.
use iron::{Chain, Handler, IronError, IronResult, Request, Response};

use error::ApiError;
use middleware::{AccessTokenAuth, MiddlewareChain};

/// The `/download/:server_name/:media_id` endpoint.
pub struct Download;

middleware_chain!(Download, [AccessTokenAuth]);

impl Handler for Download {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Err(IronError::from(ApiError::unimplemented(None)))
    }
}

/// The `/download/:server_name/:media_id/:file_name` endpoint.
pub struct DownloadFile;

middleware_chain!(DownloadFile, [AccessTokenAuth]);

impl Handler for DownloadFile {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Err(IronError::from(ApiError::unimplemented(None)))
    }
}

/// The `/upload` endpoint.
pub struct Upload;

middleware_chain!(Upload, [AccessTokenAuth]);

impl Handler for Upload {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Err(IronError::from(ApiError::unimplemented(None)))
    }
}

/// The `/thumbnail/:server_name/:media_id` endpoint.
pub struct Thumbnail;

middleware_chain!(Thumbnail, [AccessTokenAuth]);

impl Handler for Thumbnail {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Err(IronError::from(ApiError::unimplemented(None)))
    }
}
