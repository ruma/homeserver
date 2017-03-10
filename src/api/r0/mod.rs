//! API endpoints for the 0.x.x version of the Matrix spec.

pub use self::account::{
    AccountPassword,
    DeactivateAccount,
    PutAccountData,
    PutRoomAccountData,
};
pub use self::directory::{DeleteRoomAlias, GetRoomAlias, PutRoomAlias};
pub use self::event_creation::{SendMessageEvent, StateMessageEvent};
pub use self::filter::{GetFilter, PostFilter};
pub use self::join::{InviteToRoom, JoinRoom, JoinRoomWithIdOrAlias, KickFromRoom, LeaveRoom};
pub use self::login::Login;
pub use self::logout::Logout;
pub use self::members::Members;
pub use self::presence::{GetPresenceList, GetPresenceStatus, PostPresenceList, PutPresenceStatus};
pub use self::pushers::{GetPushers, SetPushers};
pub use self::profile::{GetAvatarUrl, GetDisplayName, Profile, PutAvatarUrl, PutDisplayName};
pub use self::registration::Register;
pub use self::room_creation::CreateRoom;
pub use self::room_info::{GetStateEvent, RoomState};
pub use self::sync::Sync;
pub use self::tags::{DeleteTag, GetTags, PutTag};
pub use self::versions::Versions;

mod account;
mod directory;
mod event_creation;
mod filter;
mod join;
mod login;
mod logout;
mod members;
mod presence;
mod profile;
mod pushers;
mod registration;
mod room_creation;
mod room_info;
mod sync;
mod tags;
mod versions;
