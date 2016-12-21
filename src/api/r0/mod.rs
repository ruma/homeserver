//! API endpoints for the 0.x.x version of the Matrix spec.

pub use self::account::{
    AccountPassword,
    DeactivateAccount,
    PutAccountData,
    PutRoomAccountData,
};
pub use self::directory::{GetRoomAlias, DeleteRoomAlias, PutRoomAlias};
pub use self::event_creation::{SendMessageEvent, StateMessageEvent};
pub use self::join::{InviteToRoom, JoinRoom};
pub use self::login::Login;
pub use self::logout::Logout;
pub use self::members::Members;
pub use self::profile::{Profile, GetAvatarUrl, PutAvatarUrl, GetDisplayName, PutDisplayName};
pub use self::registration::Register;
pub use self::room_creation::CreateRoom;
pub use self::tags::{DeleteTag, GetTags, PutTag};
pub use self::versions::Versions;

mod account;
mod directory;
mod event_creation;
mod join;
mod login;
mod logout;
mod members;
mod profile;
mod registration;
mod room_creation;
mod tags;
mod versions;
