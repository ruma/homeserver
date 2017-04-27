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
    DeleteTag,
    Download,
    DownloadFile,
    GetAvatarUrl,
    GetDisplayName,
    GetFilter,
    GetPresenceList,
    GetPresenceStatus,
    GetRoomAlias,
    GetTags,
    InviteToRoom,
    JoinRoom,
    JoinRoomWithIdOrAlias,
    KickFromRoom,
    LeaveRoom,
    Login,
    Logout,
    Members,
    PostFilter,
    PostPresenceList,
    PreviewUrl,
    Profile,
    PutAccountData,
    PutAvatarUrl,
    PutDisplayName,
    PutPresenceStatus,
    PutRoomAccountData,
    PutRoomAlias,
    PutTag,
    Register,
    RoomState,
    SendMessageEvent,
    StateMessageEvent,
    Sync,
    Thumbnail,
    Upload,
    Versions,
};
use config::Config;
use embedded_migrations::run as run_pending_migrations;
use error::{ApiError, CliError};
use db::DB;
use middleware::{ResponseHeaders, MiddlewareChain};
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
        debug!("Connecting to PostgreSQL.");
        let connection_pool = DB::create_connection_pool(r2d2_config, &ruma_config.postgres_url)?;
        let connection = connection_pool.get()?;

        if set_up_db {
            debug!("Setting up database.");
            setup_database(&*connection).map_err(CliError::from)?;

            debug!("Running pending database migrations.");
            run_pending_migrations(&*connection).map_err(CliError::from)?;
        }

        let mut mount = Mount::new();


        /// Add version endpoint
        let mut versions_router = Router::new();

        versions_router.get("/versions", Versions::supported(), "versions");

        let mut versions = Chain::new(versions_router);
        versions.link_after(ResponseHeaders);

        mount.mount("/_matrix/client/", versions);


        /// Add client endpoint
        let mut r0_client_router = Router::new();

        r0_client_router.post("/account/password", AccountPassword::chain(), "account_password");
        r0_client_router.post("/account/deactivate", DeactivateAccount::chain(), "deactivate_account");
        r0_client_router.post("/createRoom", CreateRoom::chain(), "create_room");
        r0_client_router.get("/directory/room/:room_alias", GetRoomAlias::chain(), "get_room_alias");
        r0_client_router.delete(
            "/directory/room/:room_alias",
            DeleteRoomAlias::chain(),
            "delete_room_alias",
        );
        r0_client_router.put("/directory/room/:room_alias", PutRoomAlias::chain(), "put_room_alias");
        r0_client_router.post("/login", Login::chain(), "login");
        r0_client_router.post("/logout", Logout::chain(), "logout");
        r0_client_router.post("/register", Register::chain(), "register");
        r0_client_router.post("/tokenrefresh", deprecated, "token_refresh");
        r0_client_router.put(
            "/user/:user_id/account_data/:type",
            PutAccountData::chain(),
            "put_account_data",
        );
        r0_client_router.put(
            "/user/:user_id/rooms/:room_id/account_data/:type",
            PutRoomAccountData::chain(),
            "put_room_account_data",
        );
        r0_client_router.put(
            "/rooms/:room_id/send/:event_type/:transaction_id",
            SendMessageEvent::chain(),
            "send_message_event",
        );
        r0_client_router.put(
            "/rooms/:room_id/state/:event_type",
            StateMessageEvent::chain(),
            "state_message_event",
        );
        r0_client_router.put(
            "/rooms/:room_id/state/:event_type/:state_key",
            StateMessageEvent::chain(),
            "state_message_event_with_key",
        );
        r0_client_router.post("/rooms/:room_id/join", JoinRoom::chain(), "join_room");
        r0_client_router.post("/rooms/:room_id/invite", InviteToRoom::chain(), "invite_to_room");
        r0_client_router.post("/join/:room_id_or_alias", JoinRoomWithIdOrAlias::chain(), "join_room_with_alias");
        r0_client_router.post("rooms/:room_id/kick", KickFromRoom::chain(), "kick_from_room");
        r0_client_router.post("rooms/:room_id/leave", LeaveRoom::chain(), "leave_room");
        r0_client_router.get("/rooms/:room_id/members", Members::chain(), "members");
        r0_client_router.get("/rooms/:room_id/state", RoomState::chain(), "get_room_state");
        r0_client_router.get("/profile/:user_id", Profile::chain(), "profile");
        r0_client_router.get("/profile/:user_id/avatar_url", GetAvatarUrl::chain(), "get_avatar_url");
        r0_client_router.get("/profile/:user_id/displayname", GetDisplayName::chain(), "get_display_name");
        r0_client_router.put("/profile/:user_id/avatar_url", PutAvatarUrl::chain(), "put_avatar_url");
        r0_client_router.put("/profile/:user_id/displayname", PutDisplayName::chain(), "put_display_name");
        r0_client_router.get("/user/:user_id/rooms/:room_id/tags", GetTags::chain(), "get_tags");
        r0_client_router.put("/user/:user_id/rooms/:room_id/tags/:tag", PutTag::chain(), "add_tag");
        r0_client_router.delete("/user/:user_id/rooms/:room_id/tags/:tag", DeleteTag::chain(), "delete_tag");
        r0_client_router.get("/user/:user_id/filter/:filter_id", GetFilter::chain(), "get_filter");
        r0_client_router.post("/user/:user_id/filter", PostFilter::chain(), "post_filter");
        r0_client_router.get("/sync", Sync::chain(), "sync");
        r0_client_router.get("/presence/:user_id/status", GetPresenceStatus::chain(), "get_presence_status");
        r0_client_router.put("/presence/:user_id/status", PutPresenceStatus::chain(), "put_presence_status");
        r0_client_router.get("/presence/list/:user_id", GetPresenceList::chain(), "get_presence_list");
        r0_client_router.post("/presence/list/:user_id", PostPresenceList::chain(), "post_presence_list");

        let mut r0_client = Chain::new(r0_client_router);

        r0_client.link_before(Read::<Config>::one(ruma_config.clone()));
        r0_client.link_before(Write::<DB>::one(connection_pool.clone()));
        r0_client.link_after(ResponseHeaders);

        mount.mount("/_matrix/client/r0/", r0_client);


        /// Add media endpoint
        let mut r0_media_router = Router::new();

        r0_media_router.get("/download/:server_name/:media_id", Download::chain(), "download");
        r0_media_router.get("/download/:server_name/:media_id/:file_name", DownloadFile::chain(), "download_file_name");
        r0_media_router.get("/thumbnail/:server_name/:media_id", Thumbnail::chain(), "thumbnail");
        r0_media_router.post("/upload", Upload::chain(), "upload");
        r0_media_router.post("/preview_url", PreviewUrl::chain(), "preview_url");

        let mut r0_media = Chain::new(r0_media_router);

        r0_media.link_before(Read::<Config>::one(ruma_config.clone()));
        r0_media.link_before(Write::<DB>::one(connection_pool));
        r0_media.link_after(ResponseHeaders);

        mount.mount("/_matrix/media/r0/", r0_media);

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

fn deprecated(_: &mut Request) -> IronResult<Response> {
    Err(IronError::from(ApiError::unauthorized("tokenrefresh is no longer supported".to_string())))
}
