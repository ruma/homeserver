use hyper::server::Listening;
use iron::{Chain, Handler, Iron};
use iron::error::HttpResult;
use iron::typemap::Key;
use mount::Mount;
use persistent::State;
use router::Router;

use api::client::r0::authentication::Register;
use repository::Repository;

struct RepositoryState;

impl Key for RepositoryState {
    type Value = Repository;
}

pub struct Server<T> where T: Handler {
    iron: Iron<T>,
}

impl Server<Mount> {
    pub fn new() -> Self {
        let mut router = Router::new();

        router.post("/register", Register::chain());

        let mut chain = Chain::new(router);

        chain.link_before(State::<RepositoryState>::one(Repository::new()));

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
