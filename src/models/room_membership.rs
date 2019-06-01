//! Matrix room membership.

use std::convert::TryInto;
use std::error::Error;

use diesel::expression::dsl::any;
use diesel::pg::data_types::PgTimestamp;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use ruma_events::room::join_rules::JoinRule;
use ruma_events::room::member::{MemberEvent, MemberEventContent, MembershipState};
use ruma_events::EventType;
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{from_value, Value};

use crate::error::ApiError;
use crate::models::event::{Event, NewEvent};
use crate::models::profile::Profile;
use crate::models::room::Room;
use crate::models::user::User;
use crate::schema::{events, room_memberships};

/// Room membership update or create data.
#[derive(Debug, Clone)]
pub struct RoomMembershipOptions {
    /// The room's ID.
    pub room_id: RoomId,
    /// The user's ID.
    pub user_id: UserId,
    /// The ID of the user who created the membership.
    pub sender: UserId,
    /// The current membership state.
    pub membership: String,
}

/// A new Matrix room membership, not yet saved.
#[derive(Debug, Clone, Insertable)]
#[table_name = "room_memberships"]
pub struct NewRoomMembership {
    /// The eventID.
    pub event_id: EventId,
    /// The room's ID.
    pub room_id: RoomId,
    /// The user's ID.
    pub user_id: UserId,
    /// The ID of the user who created the membership.
    pub sender: UserId,
    /// The current membership state.
    pub membership: String,
}

/// A Matrix room membership.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "room_memberships"]
#[primary_key(event_id)]
pub struct RoomMembership {
    /// The eventID.
    pub event_id: EventId,
    /// The room's ID.
    pub room_id: RoomId,
    /// The user's ID.
    pub user_id: UserId,
    /// The ID of the user who created the membership.
    pub sender: UserId,
    /// The current membership state.
    pub membership: String,
    /// The time the room was created.
    pub created_at: PgTimestamp,
}

impl RoomMembership {
    /// Creates a new `RoomMembership` in the database.
    pub fn create(
        connection: &PgConnection,
        homeserver_domain: &str,
        options: RoomMembershipOptions,
    ) -> Result<RoomMembership, ApiError> {
        RoomMembership::verify_creation_priviledges(connection, &options)?;

        let profile = Profile::find_by_uid(connection, &options.user_id)?;

        let new_member_event =
            RoomMembership::create_new_room_member_event(homeserver_domain, &options, profile)?;

        let new_membership = NewRoomMembership {
            event_id: new_member_event.id.clone(),
            room_id: options.room_id.clone(),
            user_id: options.user_id.clone(),
            sender: options.sender.clone(),
            membership: options.membership.clone(),
        };

        let memberships = RoomMembership::save_memberships(
            connection,
            vec![new_member_event],
            vec![new_membership],
        )?;

        Ok(memberships[0].clone())
    }

    /// Creates many `RoomMembership`s in the database.
    pub fn create_many(
        connection: &PgConnection,
        homeserver_domain: &str,
        options: Vec<RoomMembershipOptions>,
    ) -> Result<Vec<RoomMembership>, ApiError> {
        let mut events: Vec<NewEvent> = Vec::new();
        let mut new_memberships: Vec<NewRoomMembership> = Vec::new();

        for option in options {
            RoomMembership::verify_creation_priviledges(connection, &option)?;

            let profile = Profile::find_by_uid(connection, &option.user_id)?;

            let new_member_event =
                RoomMembership::create_new_room_member_event(homeserver_domain, &option, profile)?;

            let new_membership = NewRoomMembership {
                event_id: new_member_event.id.clone(),
                room_id: option.room_id.clone(),
                user_id: option.user_id.clone(),
                sender: option.sender.clone(),
                membership: option.membership.clone(),
            };

            events.push(new_member_event);
            new_memberships.push(new_membership);
        }

        RoomMembership::save_memberships(connection, events, new_memberships)
    }

    /// Save new memberships along with their corresponding `m.room.member` events.
    fn save_memberships(
        connection: &PgConnection,
        events: Vec<NewEvent>,
        new_memberships: Vec<NewRoomMembership>,
    ) -> Result<Vec<RoomMembership>, ApiError> {
        connection
            .transaction::<Vec<RoomMembership>, ApiError, _>(|| {
                diesel::insert_into(events::table)
                    .values(&events)
                    .execute(connection)
                    .map_err(ApiError::from)?;

                let memberships: Vec<RoomMembership> = diesel::insert_into(room_memberships::table)
                    .values(&new_memberships)
                    .get_results(connection)
                    .map_err(ApiError::from)?;
                Ok(memberships)
            })
            .map_err(ApiError::from)
    }

    /// Check if a `User` has enough priviledges to create a `RoomMembership`.
    fn verify_creation_priviledges(
        connection: &PgConnection,
        options: &RoomMembershipOptions,
    ) -> Result<(), ApiError> {
        let room = match Room::find(connection, &options.room_id)? {
            Some(room) => room,
            None => Err(ApiError::unauthorized(
                "The room was not found on this server".to_string(),
            ))?,
        };

        let join_rules_event = Event::find_room_join_rules_by_room_id(connection, room.id.clone())?;

        // Only the creator of the room can join an invite-only room, without an invite.
        if options.membership == "join" {
            if join_rules_event.content.join_rule == JoinRule::Invite
                && options.sender != room.user_id
            {
                return Err(ApiError::unauthorized(
                    "You are not invited to this room".to_string(),
                ));
            }

            return Ok(());
        }

        let power_levels = room.current_power_levels(connection)?;
        let user_power_level = power_levels
            .users
            .get(&options.sender)
            .unwrap_or(&power_levels.users_default);

        if options.membership == "invite" && power_levels.invite > *user_power_level {
            return Err(ApiError::unauthorized(
                "Insufficient power level to invite".to_string(),
            ));
        }
        Ok(())
    }

    /// Return `RoomMembership` for given `RoomId` and `UserId`.
    pub fn find(
        connection: &PgConnection,
        room_id: &RoomId,
        user_id: &UserId,
    ) -> Result<Option<RoomMembership>, ApiError> {
        let membership = room_memberships::table
            .filter(room_memberships::room_id.eq(room_id))
            .filter(room_memberships::user_id.eq(user_id))
            .first(connection);

        match membership {
            Ok(membership) => Ok(Some(membership)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Return `RoomMembership`'s for given `UserId`.
    pub fn find_by_uid(
        connection: &PgConnection,
        user_id: UserId,
    ) -> Result<Vec<RoomMembership>, ApiError> {
        let room_memberships: Vec<RoomMembership> = room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(err.description().to_string()),
                _ => ApiError::from(err),
            })?;

        Ok(room_memberships)
    }

    /// Update an existing `RoomMembership` entry or insert a new one.
    pub fn upsert(
        connection: &PgConnection,
        domain: &str,
        options: RoomMembershipOptions,
    ) -> Result<RoomMembership, ApiError> {
        let room_membership = RoomMembership::find(connection, &options.room_id, &options.user_id)?;

        match room_membership {
            Some(mut entry) => entry.update(connection, domain, options),
            None => RoomMembership::create(connection, domain, options),
        }
    }

    /// Update a `RoomMembership` entry using new `RoomMembershipOptions`.
    ///
    /// After the update a new `MemberEvent` is created.
    pub fn update(
        &mut self,
        connection: &PgConnection,
        homeserver_domain: &str,
        options: RoomMembershipOptions,
    ) -> Result<RoomMembership, ApiError> {
        let profile = Profile::find_by_uid(connection, &options.user_id)?;

        let event =
            RoomMembership::create_new_room_member_event(homeserver_domain, &options, profile)?;

        self.membership = options.membership.clone();
        self.sender = options.sender.clone();

        connection
            .transaction::<RoomMembership, ApiError, _>(|| {
                diesel::insert_into(events::table)
                    .values(&event)
                    .execute(connection)
                    .map_err(ApiError::from)?;

                self.save_changes::<RoomMembership>(connection)
                    .map_err(ApiError::from)?;

                // Use the new `EventId` as primary key.
                diesel::update(room_memberships::table.find(self.event_id.clone()))
                    .set(room_memberships::event_id.eq(event.id.clone()))
                    .get_result(connection)
                    .map_err(ApiError::from)
            })
            .map_err(ApiError::from)
    }

    /// Create a new `MemberEvent`.
    pub fn create_new_room_member_event(
        homeserver_domain: &str,
        options: &RoomMembershipOptions,
        profile: Option<Profile>,
    ) -> Result<NewEvent, ApiError> {
        let event_id = EventId::new(homeserver_domain).map_err(ApiError::from)?;
        let membership_string = Value::String(options.membership.clone());
        let membership: MembershipState = from_value(membership_string)?;

        let (avatar_url, displayname) = match profile {
            Some(profile) => (profile.avatar_url, profile.displayname),
            None => (None, None),
        };

        let new_member_event: NewEvent = MemberEvent {
            content: MemberEventContent {
                avatar_url,
                displayname,
                // TODO: This needs to be set based on options provided by new APIs that use it.
                is_direct: None,
                membership,
                third_party_invite: None,
            },
            event_id: event_id.clone(),
            event_type: EventType::RoomMember,
            invite_room_state: None,
            origin_server_ts: 0,
            prev_content: None,
            room_id: Some(options.room_id.clone()),
            sender: options.user_id.clone(),
            state_key: format!("@{}:{}", options.user_id.clone(), &homeserver_domain),
            unsigned: None,
        }
        .try_into()?;

        Ok(new_member_event)
    }

    /// Given a list of invited users create the appropriate membership entries and `m.room.member` events.
    pub fn create_memberships(
        connection: &PgConnection,
        room: &Room,
        invite_list: &[UserId],
        homeserver_domain: &str,
    ) -> Result<(), ApiError> {
        for invitee in invite_list {
            if invitee.hostname().to_string() != homeserver_domain {
                return Err(ApiError::unimplemented(
                    "Federation is not yet supported.".to_string(),
                ));
            }
        }
        let missing_user_ids = User::find_missing_users(connection, invite_list)?;
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

        let options = invite_list
            .iter()
            .map(|user_id| RoomMembershipOptions {
                room_id: room.id.clone(),
                user_id: user_id.clone(),
                sender: room.user_id.clone(),
                membership: "invite".to_string(),
            })
            .collect::<Vec<RoomMembershipOptions>>();

        RoomMembership::create_many(connection, homeserver_domain, options)?;

        Ok(())
    }

    /// Return member events for a given `RoomId`.
    pub fn get_events_by_room(
        connection: &PgConnection,
        room_id: RoomId,
    ) -> Result<Vec<MemberEvent>, ApiError> {
        let event_ids = room_memberships::table
            .filter(room_memberships::room_id.eq(room_id))
            .select(room_memberships::event_id);

        let events: Vec<Event> = events::table
            .filter(events::id.eq(any(event_ids)))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;

        events.into_iter().map(TryInto::try_into).collect()
    }

    /// Return all `RoomMembership`'s for given `UserId`.
    pub fn find_all_by_uid(
        connection: &PgConnection,
        user_id: &UserId,
    ) -> Result<Vec<RoomMembership>, ApiError> {
        room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Return all `RoomMembership`'s for given `UserId` and `MembershipState`.
    pub fn find_by_uid_and_state(
        connection: &PgConnection,
        user_id: UserId,
        membership: &str,
    ) -> Result<Vec<RoomMembership>, ApiError> {
        room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .filter(room_memberships::membership.eq(membership))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })
    }

    /// Return `RoomId`'s for given `UserId` and `MembershipState`.
    pub fn find_room_ids_by_uid_and_state(
        connection: &PgConnection,
        user_id: &UserId,
        membership: &str,
    ) -> Result<Vec<RoomId>, ApiError> {
        room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .filter(room_memberships::membership.eq(membership))
            .select(room_memberships::room_id)
            .get_results(connection)
            .map_err(ApiError::from)
    }

    /// Return `RoomId`'s for given `UserId`'s.
    pub fn find_common_rooms(
        connection: &PgConnection,
        user_id: &UserId,
        observed_user_id: &UserId,
        membership: &str,
    ) -> Result<Vec<RoomId>, ApiError> {
        let rooms = room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .filter(room_memberships::membership.eq(membership))
            .select(room_memberships::room_id);

        room_memberships::table
            .filter(room_memberships::user_id.eq(observed_user_id))
            .filter(room_memberships::membership.eq(membership))
            .filter(room_memberships::room_id.eq(any(rooms)))
            .select(room_memberships::room_id)
            .get_results(connection)
            .map_err(ApiError::from)
    }

    /// Filter `RoomId`'s for `UserId` and membership state.
    pub fn filter_rooms_by_state(
        connection: &PgConnection,
        room_ids: &[RoomId],
        user_id: &UserId,
        membership: &str,
    ) -> Result<Vec<RoomId>, ApiError> {
        room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .filter(room_memberships::membership.eq(membership))
            .filter(room_memberships::room_id.eq(any(room_ids)))
            .select(room_memberships::room_id)
            .get_results(connection)
            .map_err(ApiError::from)
    }
}
