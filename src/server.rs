use hyper::server::Listening;
use iron::{Handler, Iron};
use iron::error::HttpResult;
use mount::Mount;
use router::Router;

use api::client::r0::authentication;

pub struct Server<T> where T: Handler {
    iron: Iron<T>,
}

impl Server<Mount> {
    pub fn new() -> Self {
        let mut router = Router::new();

        router.post("/register", authentication::register);

        let mut mount = Mount::new();
        mount.mount("/_matrix/client/r0/", router);

        Server {
            iron: Iron::new(mount)
        }
    }

    pub fn start(self) -> HttpResult<Listening> {
        self.iron.http("localhost:3000")
    }
}
