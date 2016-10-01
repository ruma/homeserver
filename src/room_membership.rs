//! Matrix room membership.

use std::convert::TryInto;

use diesel::{
    Connection,
    ExpressionMethods,
    ExecuteDsl,
    LoadDsl,
    FilterDsl,
    SelectDsl,
    insert,
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
    MemberEventExtraContent
};
use ruma_identifiers::{EventId, RoomId, UserId};
use serde_json::{Error as SerdeJsonError, Value, from_value};

use error::ApiError;
use event::{NewEvent, Event};
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

impl RoomMembership {
    /// Creates a new room membership in the database.
    pub fn create(connection: &PgConnection,
                  homeserver_domain: &str,
                  room_membership_options: RoomMembershipOptions)
                  -> Result<RoomMembership, ApiError> {
        connection.transaction::<RoomMembership, ApiError, _>(|| {
            let room_membership = RoomMembership::find(connection, room_membership_options.clone().room_id, room_membership_options.clone().user_id)?;

            let join_rules_event = Event::find_room_join_rules_by_room_id(&connection, room_membership_options.clone().room_id)?;

            match room_membership {
                Some(room_membership) => {
                    if room_membership.membership == room_membership_options.membership {
                        return Err(ApiError::limited_rate(Some("Try to set the membership again!")));
                    }

                    Ok(room_membership)
                },
                None => {
                    if join_rules_event.content.join_rule == JoinRule::Invite {
                        return Err(ApiError::unauthorized(Some("You are not invited to this room.")));
                    }
                    let event_id = EventId::new(&homeserver_domain).map_err(ApiError::from)?;
                    let new_room_membership = NewRoomMembership {
                        event_id: event_id.clone(),
                        room_id: room_membership_options.clone().room_id,
                        user_id: room_membership_options.clone().user_id,
                        sender: room_membership_options.clone().sender,
                        membership: room_membership_options.clone().membership,
                    };

                    let membership_string = Value::String(new_room_membership.clone().membership);
                    let membership: MembershipState = from_value(membership_string)?;

                    let new_memberstate_event: NewEvent = MemberEvent {
                        content: MemberEventContent {
                            avatar_url: None,
                            displayname: None,
                            membership: membership,
                            third_party_invite: (),
                        },
                        event_id: event_id.clone(),
                        event_type: EventType::RoomMember,
                        extra_content: MemberEventExtraContent { invite_room_state: None },
                        prev_content: None,
                        room_id: room_membership_options.clone().room_id,
                        state_key: "".to_string(),
                        unsigned: None,
                        user_id: room_membership_options.clone().user_id,
                    }.try_into()?;

                    insert(&new_memberstate_event).into(events::table)
                        .execute(connection)
                        .map_err(ApiError::from)?;

                    let room_membership: RoomMembership =
                    insert(&new_room_membership).into(room_memberships::table)
                        .get_result(connection)
                        .map_err(ApiError::from)?;
                    Ok(room_membership)
                }
            }
        }).map_err(ApiError::from)
    }

    /// Return `RoomMembership` for given `RoomId` and `UserId`.
    pub fn find(connection: &PgConnection, room_id: RoomId, user_id: UserId) -> Result<Option<RoomMembership>, ApiError> {
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

        let member_events: Result<Vec<MemberEvent>, SerdeJsonError> = events.into_iter().map(TryInto::try_into).collect();
        member_events.map_err(ApiError::from)
    }
}
