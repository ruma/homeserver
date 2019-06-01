//! Storage and querying of presence lists.

use std::cmp;

use diesel::dsl::any;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use ruma_events::presence::{PresenceEvent, PresenceEventContent, PresenceState};
use ruma_events::EventType;
use ruma_identifiers::UserId;

use crate::error::ApiError;
use crate::models::presence_status::{get_now, PresenceStatus};
use crate::models::profile::Profile;
use crate::models::room_membership::RoomMembership;
use crate::models::user::User;
use crate::schema::presence_list;

/// A Matrix presence list.
#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "presence_list"]
pub struct PresenceList {
    /// Initiator.
    pub user_id: UserId,
    /// Observed user.
    pub observed_user_id: UserId,
}

impl PresenceList {
    /// Combines creations and deletions of multiple presence list entries.
    pub fn update(
        connection: &PgConnection,
        user_id: &UserId,
        invite: &[UserId],
        drop: Vec<UserId>,
    ) -> Result<(), ApiError> {
        connection
            .transaction::<(()), ApiError, _>(|| {
                let missing_user_ids = User::find_missing_users(connection, invite)?;
                if !missing_user_ids.is_empty() {
                    return Err(ApiError::bad_json(format!(
                        "Unknown users in invite list: {}",
                        &missing_user_ids
                            .iter()
                            .map(|user_id| user_id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    )));
                }

                let missing_user_ids = User::find_missing_users(connection, &drop)?;
                if !missing_user_ids.is_empty() {
                    return Err(ApiError::bad_json(format!(
                        "Unknown users in drop list: {}",
                        &missing_user_ids
                            .iter()
                            .map(|user_id| user_id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    )));
                }

                let room_ids =
                    RoomMembership::find_room_ids_by_uid_and_state(connection, user_id, "join")?;

                let mut invites: Vec<PresenceList> = Vec::new();
                for observed_user in invite {
                    if observed_user != user_id {
                        let rooms = RoomMembership::filter_rooms_by_state(
                            connection,
                            &room_ids,
                            observed_user,
                            "join",
                        )?;
                        if rooms.is_empty() {
                            return Err(ApiError::unauthorized(format!(
                                "No common rooms were found with user {}.",
                                observed_user
                            )));
                        }
                    }
                    invites.push(PresenceList {
                        user_id: user_id.clone(),
                        observed_user_id: (*observed_user).clone(),
                    });
                }
                diesel::insert_into(presence_list::table)
                    .values(&invites)
                    .execute(connection)
                    .map_err(ApiError::from)?;

                let drop = presence_list::table
                    .filter(presence_list::user_id.eq(user_id))
                    .filter(presence_list::observed_user_id.eq(any(drop)));
                diesel::delete(drop)
                    .execute(connection)
                    .map_err(ApiError::from)?;
                Ok(())
            })
            .map_err(ApiError::from)
    }

    /// Get all the `UserId`'s observed by the given `UserId`.
    pub fn find_observed_users(
        connection: &PgConnection,
        user_id: &UserId,
    ) -> Result<Vec<UserId>, ApiError> {
        let users: Vec<UserId> = presence_list::table
            .filter(presence_list::user_id.eq(user_id))
            .select(presence_list::observed_user_id)
            .get_results(connection)
            .map_err(ApiError::from)?;

        Ok(users)
    }

    /// Return `PresenceEvent`'s for given `UserId`.
    pub fn find_events_by_uid(
        connection: &PgConnection,
        user_id: &UserId,
        since: Option<i64>,
    ) -> Result<(i64, Vec<PresenceEvent>), ApiError> {
        let mut presence_key = match since {
            Some(since) => since,
            None => 0,
        };

        let observed_users = PresenceList::find_observed_users(connection, user_id)?;
        let users_status = PresenceStatus::get_users(connection, &observed_users, since)?;

        let observed_users: Vec<UserId> = users_status
            .iter()
            .map(|status| status.user_id.clone())
            .collect();
        let profiles = Profile::get_profiles(connection, &observed_users)?;

        let mut events = Vec::new();

        for status in users_status {
            let last_update = status.updated_at.0;
            presence_key = cmp::max(last_update, presence_key);

            let presence_state: PresenceState = status.presence.parse().unwrap();
            let last_active_ago = get_now() - last_update;

            let profile: Option<&Profile> =
                profiles.iter().find(|profile| profile.id == status.user_id);

            let mut avatar_url = None;
            let mut displayname = None;

            if let Some(profile) = profile {
                avatar_url = profile.avatar_url.clone();
                displayname = profile.displayname.clone();
            }

            let event = PresenceEvent {
                content: PresenceEventContent {
                    avatar_url,
                    currently_active: Some(PresenceState::Online == presence_state),
                    displayname,
                    last_active_ago: Some(last_active_ago as u64),
                    presence: presence_state,
                },
                event_type: EventType::Presence,
                sender: status.user_id,
            };

            events.push(event);
        }

        Ok((presence_key, events))
    }
}
