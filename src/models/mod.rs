pub mod access_token;
pub mod account_data;
pub mod event;
pub mod filter;
pub mod presence_list;
pub mod presence_status;
pub mod profile;
pub mod pusher;
pub mod room;
pub mod room_alias;
pub mod room_membership;
pub mod tags;
pub mod transaction;
pub mod user;

/// Helper function for skipping `false` fields when serializing with serde.
// This signature is required by Serde. Sorry, clippy.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(test: &bool) -> bool {
    !test
}
