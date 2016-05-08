use iron::{Handler, IronResult, Request, Response, status};
use iron::headers::{AccessControlAllowOrigin, ContentType};
use iron::modifiers::Header;
use mount::Mount;

#[cfg(not(feature = "swagger"))]
pub fn mount_swagger(_mount: &mut Mount) {}

#[cfg(feature = "swagger")]
pub fn mount_swagger(mount: &mut Mount) {
    struct Swagger;

    impl Handler for Swagger {
        fn handle(&self, _request: &mut Request) -> IronResult<Response> {
            let json = include_str!("swagger.json");

            Ok(Response::with((
                status::Ok,
                Header(ContentType::json()),
                Header(AccessControlAllowOrigin::Any),
                json,
            )))
        }
    }

    mount.mount("/ruma/swagger.json", Swagger);
}
