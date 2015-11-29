use hyper::server::Listening;
use iron::Iron;
use iron::error::HttpResult;
use router::Router;

use api;

pub struct Server<H> {
    iron: Iron<H>,
}

impl Server<Router> {
    pub fn new() -> Self {
        let mut router = Router::new();

        router.post("/_matrix/client/api/v2_alpha/register", api::registration::register);

        Server {
            iron: Iron::new(router)
        }
    }

    pub fn start(self) -> HttpResult<Listening> {
        self.iron.http("localhost:3000")
    }
}
