//! Iron middleware to add HTTP headers to API responses.

use iron::headers::{
    AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlAllowOrigin, Server,
};
use iron::method::Method;
use iron::{status, AfterMiddleware, IronError, IronResult, Request, Response};
use unicase::UniCase;

/// Adds a number of response headers to Ruma HTTP responses.
#[derive(Clone, Copy, Debug)]
pub struct ResponseHeaders;

/// Adds a Server header to HTTP responses
fn add_server_header(response: &mut Response) {
    response.headers.set(Server(format!(
        "{}/{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )));
}

/// Adds Cross-Origin Resource Sharing headers to HTTP responses.
fn add_cors_headers(response: &mut Response) {
    response.headers.set(AccessControlAllowHeaders(vec![
        UniCase("accept".to_string()),
        UniCase("content-type".to_string()),
    ]));
    response.headers.set(AccessControlAllowMethods(vec![
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Delete,
    ]));
    response.headers.set(AccessControlAllowOrigin::Any);
}

impl AfterMiddleware for ResponseHeaders {
    fn after(&self, request: &mut Request<'_, '_>, mut response: Response) -> IronResult<Response> {
        if request.method == Method::Options {
            response = Response::with(status::Ok);
        }
        add_server_header(&mut response);
        add_cors_headers(&mut response);

        Ok(response)
    }

    fn catch(&self, _: &mut Request<'_, '_>, mut error: IronError) -> IronResult<Response> {
        add_server_header(&mut error.response);
        add_cors_headers(&mut error.response);

        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{Response, Test};
    use iron::headers::{
        AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlAllowOrigin, Server,
    };
    use iron::method::Method;
    use unicase::UniCase;

    fn check_for_modified_headers(response: &Response) {
        assert_eq!(
            response.headers.get::<Server>().unwrap(),
            &Server(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
        );
        assert_eq!(
            response.headers.get::<AccessControlAllowHeaders>().unwrap(),
            &AccessControlAllowHeaders(vec![
                UniCase("accept".to_string()),
                UniCase("content-type".to_string())
            ])
        );
        assert_eq!(
            response.headers.get::<AccessControlAllowMethods>().unwrap(),
            &AccessControlAllowMethods(vec![
                Method::Get,
                Method::Post,
                Method::Put,
                Method::Delete
            ])
        );
        assert_eq!(
            response.headers.get::<AccessControlAllowOrigin>().unwrap(),
            &AccessControlAllowOrigin::Any
        );
    }

    #[test]
    fn versions_response_headers() {
        let test = Test::new();
        let response = test.get("/_matrix/client/versions");

        // Check to see if the expected headers have been added to the response.
        check_for_modified_headers(&response);
    }

    #[test]
    fn r0_response_headers() {
        let test = Test::new();
        let response = test.get("/_matrix/client/r0/events");

        // Check to see if the expected headers have been added to the response.
        check_for_modified_headers(&response);
    }

    #[test]
    fn swagger_response_headers() {
        let test = Test::new();
        let response = test.get("/ruma/swagger.json ");

        // Check to see if the expected headers have been added to the response.
        check_for_modified_headers(&response);
    }
}
