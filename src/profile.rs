//! Matrix profile.

use diesel::{
    ExpressionMethods,
    LoadDsl,
    FilterDsl,
    SaveChangesDsl,
    insert,
};
use diesel::pg::PgConnection;
use diesel::result::Error as DieselError;
use ruma_identifiers::UserId;

use error::ApiError;
use room_membership::{RoomMembership, RoomMembershipOptions};
use schema::profiles;

/// A new Matrix profile, not yet saved.
#[derive(Debug, Clone, Insertable)]
#[table_name = "profiles"]
pub struct NewProfile {
    /// The user's ID.
    pub id: UserId,
    /// The avatar url.
    pub avatar_url: Option<String>,
    /// The display name.
    pub displayname: Option<String>,
}

/// A Matrix profile.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "profiles"]
pub struct Profile {
    /// The user's ID.
    pub id: UserId,
    /// The avatar url.
    pub avatar_url: Option<String>,
    /// The display name.
    pub displayname: Option<String>,
}

impl Profile {
    /// Update or Create a `Profile` entry with new avatar_url.
    pub fn update_avatar_url(connection: &PgConnection, user_id: UserId, avatar_url: Option<String>)
    -> Result<Profile, ApiError> {
        let profile = Profile::find_by_uid(connection, user_id.clone())?;

        match profile {
            Some(mut profile) => profile.set_avatar_url(connection, avatar_url),
            None => {
                let new_profile = Profile {
                    id: user_id.clone(),
                    avatar_url: avatar_url,
                    displayname: None,
                };

                Profile::create(connection, &new_profile)
            }
        }
    }

    /// Update or Create a `Profile` entry with new displayname.
    pub fn update_displayname(connection: &PgConnection, user_id: UserId, displayname: Option<String>)
    -> Result<Profile, ApiError> {
        let profile = Profile::find_by_uid(connection, user_id.clone())?;

        match profile {
            Some(mut profile) => profile.set_displayname(connection, displayname),
            None => {
                let new_profile = Profile {
                    id: user_id.clone(),
                    avatar_url: None,
                    displayname: displayname,
                };

                Profile::create(connection, &new_profile)
            }
        }
    }

    /// Update a `Profile` entry with new avatar_url.
    fn set_avatar_url(&mut self, connection: &PgConnection, avatar_url: Option<String>)
    -> Result<Profile, ApiError> {
        self.avatar_url = avatar_url;

        match self.save_changes::<Profile>(connection) {
            Ok(_) => Ok(self.clone()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Update a `Profile` entry with new displayname.
    fn set_displayname(&mut self, connection: &PgConnection, displayname: Option<String>)
    -> Result<Profile, ApiError> {
        self.displayname = displayname;

        match self.save_changes::<Profile>(connection) {
            Ok(_) => Ok(self.clone()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Update `RoomMembership`'s due to changed `Profile`.
    pub fn update_memberships(connection: &PgConnection, homeserver_domain: &str, user_id: UserId)
    -> Result<(), ApiError> {
        let mut room_memberships = RoomMembership::find_by_uid(connection, user_id.clone())?;

        for room_membership in room_memberships.iter_mut() {
            let options = RoomMembershipOptions {
                room_id: room_membership.room_id.clone(),
                user_id: user_id.clone(),
                sender: user_id.clone(),
                membership: "join".to_string(),
            };

            room_membership.update(connection, homeserver_domain, options)?;
        }

        Ok(())
    }

    /// Create a `Profile` entry.
    pub fn create(connection: &PgConnection, new_profile: &Profile) -> Result<Profile, ApiError> {
        insert(new_profile)
            .into(profiles::table)
            .get_result(connection)
            .map_err(ApiError::from)
    }

    /// Return `Profile` for given `UserId`.
    pub fn find_by_uid(connection: &PgConnection, user_id: UserId) -> Result<Option<Profile>, ApiError> {
        let profile = profiles::table
            .filter(profiles::id.eq(user_id))
            .first(connection);

        match profile {
            Ok(profile) => Ok(Some(profile)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }
}
