use diesel::migrations::run_pending_migrations;
use hyper::server::Listening;
use iron::{Chain, Handler, Iron};
use iron::error::HttpResult;
use mount::Mount;
use persistent::Write;
use r2d2::{Config as R2D2Config, Pool};
use r2d2_diesel::ConnectionManager;
use router::Router;

use api::r0::authentication::Register;
use api::r0::versions::Versions;
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

        debug!("Connecting to PostgreSQL.");
        let r2d2_config = R2D2Config::default();
        let connection_manager = ConnectionManager::new(&config.postgres_url[..]);
        let connection_pool = try!(Pool::new(r2d2_config, connection_manager));
        let connection = try!(connection_pool.get());

        debug!("Running pending migrations.");
        match run_pending_migrations(&connection) {
            Ok(_) => {},
            Err(error) => return Err(CLIError::new(format!("{:?}", error))),
        }

        chain.link_before(Write::<DB>::one(connection_pool));

        let mut versions = Router::new();
        versions.get("/versions", Versions::new(vec!["r0.0.1"]));

        let mut mount = Mount::new();

        mount.mount("/_matrix/client/", versions);
        mount.mount("/_matrix/client/r0/", chain);

        Ok(Server {
            iron: Iron::new(mount),
        })
    }

    pub fn start(self) -> HttpResult<Listening> {
        info!("Starting Ruma server on localhost:3000.");

        self.iron.http("localhost:3000")
    }
}
