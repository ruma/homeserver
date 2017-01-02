//! Matrix room membership.

use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::error::Error;

use diesel::{
    Connection,
    ExpressionMethods,
    ExecuteDsl,
    FilterDsl,
    FindDsl,
    LoadDsl,
    OrderDsl,
    SaveChangesDsl,
    SelectDsl,
    insert,
    update,
};
use diesel::expression::dsl::*;
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::Error as DieselError;
use ruma_events::EventType;
use ruma_events::room::join_rules::JoinRule;
use ruma_events::room::member::{
    MemberEvent,
    MembershipState,
    MemberEventContent,
};
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{Value, from_value};

use error::ApiError;
use models::event::{NewEvent, Event};
use models::user::User;
use models::profile::Profile;
use models::room::Room;
use schema::{events, users, room_memberships};

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
    pub fn create(connection: &PgConnection, homeserver_domain: &str, options: RoomMembershipOptions)
    -> Result<RoomMembership, ApiError> {
        let room = Room::find(connection, &options.room_id)?;

        RoomMembership::verify_creation_priviledges(
            connection,
            options.sender.clone(),
            &options.membership,
            &room
        )?;

        let profile = Profile::find_by_uid(connection, options.user_id.clone())?;

        let new_member_event = RoomMembership::create_new_room_member_event(
            homeserver_domain,
            &options,
            profile,
        )?;

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
            vec![new_membership]
        )?;

        Ok(memberships.get(0).unwrap().clone())
    }

    /// Creates many `RoomMembership`s in the database.
    pub fn create_many(connection: &PgConnection, homeserver_domain: &str, options: Vec<RoomMembershipOptions>)
    -> Result<Vec<RoomMembership>, ApiError> {
        let mut events: Vec<NewEvent> = Vec::new();
        let mut new_memberships: Vec<NewRoomMembership> = Vec::new();

        for option in options {
            let room = Room::find(connection, &option.room_id)?;

            RoomMembership::verify_creation_priviledges(
                connection,
                option.sender.clone(),
                &option.membership,
                &room
            )?;

            let profile = Profile::find_by_uid(connection, option.user_id.clone())?;

            let new_member_event = RoomMembership::create_new_room_member_event(
                homeserver_domain,
                &option,
                profile,
            )?;

            let new_membership = NewRoomMembership {
                event_id: new_member_event.id.clone(),
                room_id: room.id.clone(),
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
    fn save_memberships(connection: &PgConnection, events: Vec<NewEvent>, new_memberships: Vec<NewRoomMembership>)
    -> Result<Vec<RoomMembership>, ApiError> {
        connection.transaction::<Vec<RoomMembership>, ApiError, _>(|| {
            insert(&events)
                .into(events::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            let memberships: Vec<RoomMembership> = insert(&new_memberships)
                                                    .into(room_memberships::table)
                                                    .get_results(connection)
                                                    .map_err(ApiError::from)?;
            Ok(memberships)
        }).map_err(ApiError::from)
    }

    /// Check if a `User` has enough priviledges to create a `RoomMembership`.
    fn verify_creation_priviledges(
        connection: &PgConnection,
        sender: UserId,
        membership: &str,
        room: &Room
    ) -> Result<(), ApiError> {
        let join_rules_event = Event::find_room_join_rules_by_room_id(connection, room.id.clone())?;

        // Only the creator of the room can join an invite-only room, without an invite.
        if sender != room.user_id {
            if membership == "join" && join_rules_event.content.join_rule == JoinRule::Invite {
                return Err(ApiError::unauthorized("You are not invited to this room".to_string()));
            }

            let power_levels = room.current_power_levels(connection)?;
            let user_power_level = power_levels
                .users
                .get(&sender)
                .unwrap_or(&power_levels.users_default);

            if membership == "invite" {
                if power_levels.invite > *user_power_level {
                    return Err(ApiError::unauthorized("Insufficient power level to invite".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Return `RoomMembership` for given `RoomId` and `UserId`.
    pub fn find(connection: &PgConnection, room_id: &RoomId, user_id: &UserId)
    -> Result<Option<RoomMembership>, ApiError> {
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
    pub fn find_by_uid(connection: &PgConnection, user_id: UserId) -> Result<Vec<RoomMembership>, ApiError> {
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
    pub fn upsert(connection: &PgConnection, domain: &str, options: RoomMembershipOptions)
    -> Result<RoomMembership, ApiError> {
        let room_membership = RoomMembership::find(
            connection,
            &options.room_id,
            &options.user_id
        )?;

        match room_membership {
            Some(mut entry) => entry.update(connection, domain, options),
            None => RoomMembership::create(connection, domain, options)
        }
    }

    /// Update a `RoomMembership` entry using new `RoomMembershipOptions`.
    ///
    /// After the update a new `MemberEvent` is created.
    pub fn update(&mut self, connection: &PgConnection, homeserver_domain: &str, options: RoomMembershipOptions)
    -> Result<RoomMembership, ApiError> {
        let profile = Profile::find_by_uid(connection, options.user_id.clone())?;

        let event = RoomMembership::create_new_room_member_event(
            &homeserver_domain,
            &options,
            profile,
        )?;

        self.membership = options.membership.clone();
        self.sender = options.sender.clone();

        connection.transaction::<RoomMembership, ApiError, _>(|| {
            insert(&event)
                .into(events::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            self.save_changes::<RoomMembership>(connection)
                .map_err(ApiError::from)?;

            // Use the new `EventId` as primary key.
            update(room_memberships::table.find(self.event_id.clone()))
                .set(room_memberships::event_id.eq(event.id.clone()))
                .get_result(connection)
                .map_err(ApiError::from)
        }).map_err(ApiError::from)
    }

    /// Create a new `MemberEvent`.
    pub fn create_new_room_member_event(
        homeserver_domain: &str,
        options: &RoomMembershipOptions,
        profile: Option<Profile>
    ) -> Result<NewEvent, ApiError> {
        let event_id = EventId::new(&homeserver_domain).map_err(ApiError::from)?;
        let membership_string = Value::String(options.membership.clone());
        let membership: MembershipState = from_value(membership_string)?;

        let (avatar_url, displayname) = match profile {
            Some(profile) => (profile.avatar_url, profile.displayname),
            None => (None, None),
        };

        let new_member_event: NewEvent = MemberEvent {
            content: MemberEventContent {
                avatar_url: avatar_url,
                displayname: displayname,
                membership: membership,
                third_party_invite: None,
            },
            event_id: event_id.clone(),
            event_type: EventType::RoomMember,
            invite_room_state: None,
            prev_content: None,
            room_id: options.room_id.clone(),
            state_key: format!("@{}:{}", options.user_id.clone(), &homeserver_domain),
            unsigned: None,
            user_id: options.user_id.clone(),
        }.try_into()?;

        Ok(new_member_event)
    }

    /// Given a list of invited users create the appropriate membership entries and `m.room.member` events.
    pub fn create_memberships(
        connection: &PgConnection,
        room: &Room,
        invite_list: &Vec<String>,
        homeserver_domain: &str
    ) -> Result<(), ApiError> {
        let mut user_ids = HashSet::with_capacity(invite_list.len());

        for invitee in invite_list {
            let user_id = UserId::try_from(invitee)?;

            if user_id.hostname().to_string() != homeserver_domain {
                return Err(
                    ApiError::unimplemented("Federation is not yet supported.".to_string())
                );
            }

            user_ids.insert(user_id);
        }

        let users: Vec<User> = users::table
            .filter(users::id.eq(any(
                user_ids.iter().cloned().collect::<Vec<UserId>>()))
            )
            .get_results(connection)
            .map_err(ApiError::from)?;

        let loaded_user_ids: HashSet<UserId> = users
            .iter()
            .map(|user| user.id.clone())
            .collect();

        let missing_user_ids: Vec<UserId> = user_ids
            .difference(&loaded_user_ids)
            .cloned()
            .collect();

        if missing_user_ids.len() > 0 {
            return Err(
                ApiError::bad_json(format!(
                    "Unknown users in invite list: {}",
                    &missing_user_ids
                        .iter()
                        .map(|user_id| user_id.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                ))
            )
        }

        let options = users.iter().map(|user| {
            RoomMembershipOptions {
                room_id: room.id.clone(),
                user_id: user.id.clone(),
                sender: room.user_id.clone(),
                membership: "invite".to_string(),
            }
        }).collect::<Vec<RoomMembershipOptions>>();

        RoomMembership::create_many(connection, &homeserver_domain, options)?;

        Ok(())
    }

    /// Return member events for given `room_id`.
    pub fn get_events_by_room(connection: &PgConnection, room_id: RoomId) -> Result<Vec<MemberEvent>, ApiError> {
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

        events.into_iter() .map(TryInto::try_into). collect()
    }

    /// Return `RoomMembership`'s for given `UserId` order by `RoomId`.
    pub fn find_by_user_id_order_by_room_id(connection: &PgConnection, user_id: &UserId) -> Result<Vec<RoomMembership>, ApiError> {
        let room_memberships: Vec<RoomMembership> = room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .order(room_memberships::room_id)
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;
        Ok(room_memberships)
    }

    /// Return `RoomMembership`'s for given `UserId` and `MembershipState`.
    pub fn find_by_uid_and_state(connection: &PgConnection, user_id: UserId, membership: &str) -> Result<Vec<RoomMembership>, ApiError> {
        let room_memberships: Vec<RoomMembership> = room_memberships::table
            .filter(room_memberships::user_id.eq(user_id))
            .filter(room_memberships::membership.eq(membership))
            .get_results(connection)
            .map_err(|err| match err {
                DieselError::NotFound => ApiError::not_found(None),
                _ => ApiError::from(err),
            })?;
        Ok(room_memberships)
    }
}
