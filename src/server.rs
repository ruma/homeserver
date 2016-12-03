//! Iron web server that serves the API.
use diesel::migrations::setup_database;
use diesel::pg::PgConnection;
use iron::{Chain, Iron, IronError, IronResult, Listening, Request, Response};
use iron::error::HttpResult;
use mount::Mount;
use persistent::{Read, Write};
use r2d2::Config as R2D2Config;
use r2d2_diesel::Error as R2D2DieselError;
use router::Router;

use api::r0::{
    AccountPassword,
    CreateRoom,
    DeactivateAccount,
    DeleteRoomAlias,
    GetAvatarUrl,
    GetDisplayName,
    GetRoomAlias,
    InviteToRoom,
    JoinRoom,
    Login,
    Logout,
    Members,
    Profile,
    PutAccountData,
    PutAvatarUrl,
    PutDisplayName,
    PutRoomAccountData,
    PutRoomAlias,
    Register,
    SendMessageEvent,
    StateMessageEvent,
    Versions,
};
use config::Config;
use embedded_migrations::run as run_pending_migrations;
use error::{ApiError, CliError};
use db::DB;
use middleware::{Cors, MiddlewareChain};
use swagger::mount_swagger;

/// Ruma's web server.
pub struct Server<'a> {
    config: &'a Config,
    mount: Mount,
}

impl<'a> Server<'a> {
    /// Create a new `Server` from a `Config`.
    pub fn new(config: &Config)
    -> Result<Server, CliError> {
        Server::with_options(config, R2D2Config::default(), true)
    }

    /// Create a new `Server` from a `Config`, an `r2d2::Config`, and the ability to disable
    /// database creation and setup.
    pub fn with_options(
        ruma_config: &Config,
        r2d2_config: R2D2Config<PgConnection, R2D2DieselError>,
        set_up_db: bool,
    ) -> Result<Server, CliError> {
        let mut r0_router = Router::new();

        r0_router.post("/account/password", AccountPassword::chain(), "account_password");
        r0_router.post("/account/deactivate", DeactivateAccount::chain(), "deactivate_account");
        r0_router.post("/createRoom", CreateRoom::chain(), "create_room");
        r0_router.get("/directory/room/:room_alias", GetRoomAlias::chain(), "get_room_alias");
        r0_router.delete(
            "/directory/room/:room_alias",
            DeleteRoomAlias::chain(),
            "delete_room_alias",
        );
        r0_router.put("/directory/room/:room_alias", PutRoomAlias::chain(), "put_room_alias");
        r0_router.post("/login", Login::chain(), "login");
        r0_router.post("/logout", Logout::chain(), "logout");
        r0_router.post("/register", Register::chain(), "register");
        r0_router.post("/tokenrefresh", unimplemented, "token_refresh");
        r0_router.put(
            "/user/:user_id/account_data/:type",
            PutAccountData::chain(),
            "put_account_data",
        );
        r0_router.put(
            "/user/:user_id/rooms/:room_id/account_data/:type",
            PutRoomAccountData::chain(),
            "put_room_account_data",
        );
        r0_router.put(
            "/rooms/:room_id/send/:event_type/:transaction_id",
            SendMessageEvent::chain(),
            "send_message_event",
        );
        r0_router.put(
            "/rooms/:room_id/state/:event_type",
            StateMessageEvent::chain(),
            "state_message_event",
        );
        r0_router.put(
            "/rooms/:room_id/state/:event_type/:state_key",
            StateMessageEvent::chain(),
            "state_message_event_with_key",
        );
        r0_router.post("/rooms/:room_id/join", JoinRoom::chain(), "join_room");
        r0_router.post("/rooms/:room_id/invite", InviteToRoom::chain(), "invite_to_room");
        r0_router.get("/rooms/:room_id/members", Members::chain(), "members");
        r0_router.get("/profile/:user_id", Profile::chain(), "profile");
        r0_router.get("/profile/:user_id/avatar_url", GetAvatarUrl::chain(), "get_avatar_url");
        r0_router.get("/profile/:user_id/displayname", GetDisplayName::chain(), "get_display_name");
        r0_router.put("/profile/:user_id/avatar_url", PutAvatarUrl::chain(), "put_avatar_url");
        r0_router.put("/profile/:user_id/displayname", PutDisplayName::chain(), "put_display_name");

        let mut r0 = Chain::new(r0_router);

        debug!("Connecting to PostgreSQL.");
        let connection_pool = DB::create_connection_pool(r2d2_config, &ruma_config.postgres_url)?;
        let connection = connection_pool.get()?;

        if set_up_db {
            debug!("Setting up database.");
            if let Err(error) =  setup_database(&*connection) {
                return Err(CliError::new(format!("{:?}", error)));
            }

            debug!("Running pending database migrations.");
            if let Err(error) = run_pending_migrations(&*connection) {
                return Err(CliError::new(format!("{:?}", error)));
            }
        }

        r0.link_before(Read::<Config>::one(ruma_config.clone()));
        r0.link_before(Write::<DB>::one(connection_pool));
        r0.link_after(Cors);

        let mut versions_router = Router::new();

        versions_router.get("/versions", Versions::new(vec!["r0.0.1"]), "versions");

        let mut versions = Chain::new(versions_router);

        versions.link_after(Cors);

        let mut mount = Mount::new();

        mount.mount("/_matrix/client/", versions);
        mount.mount("/_matrix/client/r0/", r0);

        mount_swagger(&mut mount);

        Ok(Server {
            config: ruma_config,
            mount: mount,
        })
    }

    /// Run the server and block the current thread until stopped or interrupted.
    pub fn run(self) -> HttpResult<Listening> {
        let address = format!("{}:{}", self.config.bind_address, self.config.bind_port);

        info!("Starting Ruma server on {}.", address);

        let iron = Iron::new(self.mount);

        iron.http(&address[..])
    }

    /// Moves out the server's `Mount`. Useful for testing.
    pub fn into_mount(self) -> Mount {
        self.mount
    }
}

fn unimplemented(_request: &mut Request) -> IronResult<Response> {
    let error = ApiError::unimplemented(None);

    Err(IronError::new(error.clone(), error))
}
