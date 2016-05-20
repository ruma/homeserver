use iron::{AfterMiddleware, IronError, IronResult, Request, Response, status};
use iron::headers::{
    AccessControlAllowHeaders,
    AccessControlAllowMethods,
    AccessControlAllowOrigin,
};
use iron::method::Method;
use unicase::UniCase;

/// Adds Cross-Origin Resource Sharing headers to HTTP responses.
pub struct Cors;

fn add_headers(response: &mut Response) {
    response.headers.set(AccessControlAllowHeaders(
            vec![UniCase("accept".to_string()), UniCase("content-type".to_string())]
    ));
    response.headers.set(AccessControlAllowMethods(
            vec![Method::Get, Method::Post, Method::Put, Method::Delete]
    ));
    response.headers.set(AccessControlAllowOrigin::Any);
}

impl AfterMiddleware for Cors {
    fn after(&self, request: &mut Request, mut response: Response) -> IronResult<Response> {
        if request.method == Method::Options {
            response = Response::with(status::Ok);
        }

        add_headers(&mut response);

        Ok(response)
    }

    fn catch(&self, _: &mut Request, mut error: IronError) -> IronResult<Response> {
        add_headers(&mut error.response);

        Err(error)
    }
}
