use diesel::Connection;
use hyper::server::Listening;
use iron::{Chain, Handler, Iron};
use iron::error::HttpResult;
use mount::Mount;
use persistent::Write;
use router::Router;

use api::r0::authentication::Register;
use config::Config;
use db::DB;

pub struct Server<T> where T: Handler {
    iron: Iron<T>,
}

impl Server<Mount> {
    pub fn new(config: &Config) -> Self {
        let mut router = Router::new();

        router.post("/register", Register::chain());

        let mut chain = Chain::new(router);

        chain.link_before(
            Write::<DB>::one(
                Connection::establish(&config.postgres_url).expect(
                    "failed to establish connection to PostgreSQL"
                )
            )
        );

        let mut mount = Mount::new();

        mount.mount("/_matrix/client/r0/", chain);

        Server {
            iron: Iron::new(mount),
        }
    }

    pub fn start(self) -> HttpResult<Listening> {
        self.iron.http("localhost:3000")
    }
}
