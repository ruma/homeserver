//! Matrix room membership.

use std::convert::TryInto;
use std::error::Error;

use diesel::{
    Connection,
    ExpressionMethods,
    ExecuteDsl,
    LoadDsl,
    FilterDsl,
    FindDsl,
    SaveChangesDsl,
    SelectDsl,
    insert,
    update,
};
use diesel::associations::Identifiable;
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
    MemberEventExtraContent
};
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{Error as SerdeJsonError, Value, from_value};

use error::ApiError;
use event::{NewEvent, Event};
use profile::Profile;
use room::Room;
use schema::{events, room_memberships};

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
#[derive(Debug, Clone)]
#[insertable_into(room_memberships)]
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
#[derive(Debug, Clone, Queryable)]
#[changeset_for(room_memberships)]
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

impl Identifiable for RoomMembership {
    type Id = EventId;
    type Table = room_memberships::table;

    fn id(&self) -> &Self::Id {
        &self.event_id
    }

    fn table() -> Self::Table {
        room_memberships::table
    }
}

impl RoomMembership {
    /// Creates a new `RoomMembership` in the database.
    pub fn create(connection: &PgConnection, homeserver_domain: &str, options: RoomMembershipOptions)
    -> Result<RoomMembership, ApiError> {
        connection.transaction::<RoomMembership, ApiError, _>(|| {
            let join_rules_event = Event::find_room_join_rules_by_room_id(
                &connection,
                options.clone().room_id
            )?;

            let room = Room::find(connection, &options.room_id)?;

            // Only the creator of the room can join an invite-only room,
            // without an invite.
            if options.sender != room.user_id {
                if options.membership == "join" && join_rules_event.content.join_rule == JoinRule::Invite {
                    return Err(ApiError::unauthorized(Some("You are not invited to this room")));
                }

                let power_levels = room.current_power_levels(connection)?;
                let user_power_level = power_levels
                    .users
                    .get(&options.sender)
                    .unwrap_or(&power_levels.users_default);

                if options.membership == "invite" {
                    if power_levels.invite > *user_power_level {
                        return Err(
                            ApiError::unauthorized(Some("Insufficient power level to invite"))
                        );
                    }
                }
            }

            let profile = Profile::find_by_uid(connection, options.user_id.clone())?;

            let new_member_event = RoomMembership::create_new_room_member_event(
                homeserver_domain,
                &options,
                profile
            )?;

            let new_room_membership = NewRoomMembership {
                event_id: new_member_event.id.clone(),
                room_id: options.room_id.clone(),
                user_id: options.user_id.clone(),
                sender: options.sender.clone(),
                membership: options.membership.clone(),
            };

            insert(&new_member_event)
                .into(events::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            let room_membership: RoomMembership = insert(&new_room_membership)
                                                    .into(room_memberships::table)
                                                    .get_result(connection)
                                                    .map_err(ApiError::from)?;

            Ok(room_membership)
        }).map_err(ApiError::from)
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
                DieselError::NotFound => ApiError::not_found(Some(err.description())),
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
                third_party_invite: (),
            },
            event_id: event_id.clone(),
            event_type: EventType::RoomMember,
            extra_content: MemberEventExtraContent { invite_room_state: None },
            prev_content: None,
            room_id: options.room_id.clone(),
            state_key: format!("@{}:{}", options.user_id.clone(), &homeserver_domain),
            unsigned: None,
            user_id: options.user_id.clone(),
        }.try_into()?;

        Ok(new_member_event)
    }

    /// Return member event's for given `room_id`.
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

        let member_events: Result<Vec<MemberEvent>, SerdeJsonError> = events.into_iter()
                                                                        .map(TryInto::try_into)
                                                                        .collect();
        member_events.map_err(ApiError::from)
    }
}
