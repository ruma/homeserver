//! Iron web server that serves the API.
use diesel::migrations::{run_pending_migrations, setup_database};
use hyper::server::Listening;
use iron::{Chain, Handler, Iron};
use iron::error::HttpResult;
use mount::Mount;
use persistent::{Read, Write};
use r2d2::{Config as R2D2Config, Pool};
use r2d2_diesel::ConnectionManager;
use router::Router;

use api::r0::registration::Register;
use api::r0::versions::Versions;
use config::FinalConfig;
use error::CLIError;
use db::DB;
use swagger::mount_swagger;

/// Ruma's web server.
pub struct Server<'a, T> where T: Handler {
    config: &'a FinalConfig,
    iron: Iron<T>,
}

impl<'a> Server<'a, Mount> {
    /// Create a new `Server` from a `FinalConfig`.
    pub fn new(config: &FinalConfig) -> Result<Server<Mount>, CLIError> {
        let mut router = Router::new();

        router.post("/register", Register::chain());

        let mut r0 = Chain::new(router);

        debug!("Connecting to PostgreSQL.");
        let r2d2_config = R2D2Config::default();
        let connection_manager = ConnectionManager::new(&config.postgres_url[..]);
        let connection_pool = try!(Pool::new(r2d2_config, connection_manager));
        let connection = try!(connection_pool.get());

        debug!("Setting up database.");
        if let Err(error) =  setup_database(&*connection) {
            return Err(CLIError::new(format!("{:?}", error)));
        }

        debug!("Running pending database migrations.");
        if let Err(error) = run_pending_migrations(&*connection) {
            return Err(CLIError::new(format!("{:?}", error)));
        }

        r0.link_before(Read::<FinalConfig>::one(config.clone()));
        r0.link_before(Write::<DB>::one(connection_pool));

        let mut versions = Router::new();
        versions.get("/versions", Versions::new(vec!["r0.0.1"]));

        let mut mount = Mount::new();

        mount.mount("/_matrix/client/", versions);
        mount.mount("/_matrix/client/r0/", r0);

        mount_swagger(&mut mount);

        Ok(Server {
            config: config,
            iron: Iron::new(mount),
        })
    }

    /// Run the server and block the current thread until stopped or interrupted.
    pub fn run(self) -> HttpResult<Listening> {
        let address = format!("{}:{}", self.config.bind_address, self.config.bind_port);

        info!("Starting Ruma server on {}.", address);

        self.iron.http(&address[..])
    }
}
