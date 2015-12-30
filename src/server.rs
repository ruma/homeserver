use diesel::Connection;
use hyper::server::Listening;
use iron::{Chain, Handler, Iron};
use iron::error::HttpResult;
use mount::Mount;
use persistent::Write;
use router::Router;

use api::r0::authentication::Register;
use config::Config;
use error::CLIError;
use db::DB;

pub struct Server<T> where T: Handler {
    iron: Iron<T>,
}

impl Server<Mount> {
    pub fn new(config: &Config) -> Result<Server<Mount>, CLIError> {
        let mut router = Router::new();

        router.post("/register", Register::chain());

        let mut chain = Chain::new(router);

        chain.link_before(Write::<DB>::one(try!(Connection::establish(&config.postgres_url))));

        let mut mount = Mount::new();

        mount.mount("/_matrix/client/r0/", chain);

        Ok(Server {
            iron: Iron::new(mount),
        })
    }

    pub fn start(self) -> HttpResult<Listening> {
        self.iron.http("localhost:3000")
    }
}
