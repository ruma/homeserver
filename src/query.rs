//! Matrix sync.

use std::cmp;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::i64;
use std::iter::Iterator;
use std::str::FromStr;

use diesel::pg::PgConnection;
use ruma_events::collections::all::{RoomEvent, StateEvent};
use ruma_events::presence::PresenceEvent;
use ruma_events::presence::PresenceState;
use ruma_events::stripped::StrippedState;
use ruma_events::EventType;
use ruma_identifiers::RoomId;
use serde_json::Value;

use crate::error::ApiError;
use crate::models::event::Event;
use crate::models::filter::{ContentFilter, RoomEventFilter, RoomFilter};
use crate::models::presence_list::PresenceList;
use crate::models::presence_status::PresenceStatus;
use crate::models::room_membership::RoomMembership;
use crate::models::user::User;

/// Counts of unread notifications for a room.
#[derive(Debug, Clone, Serialize)]
struct UnreadNotificationCounts {
    /// The number of unread notifications for a room with the highlight flag set.
    highlight_count: u64,
    /// The total number of unread notifications for a room.
    notification_count: u64,
}

/// A timeline of events.
#[derive(Debug, Clone, Serialize)]
struct Timeline {
    /// List of events.
    events: Vec<RoomEvent>,
    /// True if the number of events returned was limited by the limit on the filter.
    limited: bool,
    /// A token that can be supplied to to the from parameter of the `rooms/{roomId}/messages` endpoint.
    prev_batch: String,
}

/// Generic placeholder for the different event types.
#[derive(Debug, Clone, Serialize)]
struct Events<T> {
    /// A list of events.
    events: Vec<T>,
}

/// Information about rooms the user has left or been banned from.
#[derive(Debug, Clone, Serialize)]
struct LeftRoom {
    /// The state updates for the room up to the start of the timeline.
    state: Events<StateEvent>,
    /// The timeline of messages and state changes in the room up to the point when the user left.
    timeline: Timeline,
}

/// Information about the rooms the user has been invited to.
#[derive(Debug, Clone, Serialize)]
struct InvitedRoom {
    /// The state of a room that the user has been invited to.
    invite_state: Events<StrippedState>,
}

/// Information about the rooms the user has joined.
#[derive(Debug, Clone, Serialize)]
struct JoinedRoom {
    /// Counts of unread notifications for this room.
    unread_notifications: UnreadNotificationCounts,
    /// The timeline of messages and state changes in the room.
    timeline: Timeline,
    /// Updates to the state, between the time indicated by the since parameter,
    /// and the start of the timeline (or all state up to the start of the timeline,
    /// if since is not given, or full_state is true).
    state: Events<StateEvent>,
    /// The private data that this user has attached to this room.
    account_data: Events<Value>,
    /// The ephemeral events in the room that aren't recorded in the timeline or
    /// state of the room. e.g. typing.
    ephemeral: Events<Value>,
}

/// Information about rooms the user has joined, been invited to, or left.
#[derive(Debug, Clone, Serialize)]
struct Rooms {
    /// The rooms that the user has been invited to.
    invite: HashMap<RoomId, InvitedRoom>,
    /// The rooms that the user has joined.
    join: HashMap<RoomId, JoinedRoom>,
    /// The rooms that the user has left or been banned from.
    leave: HashMap<RoomId, LeftRoom>,
}

/// A Sync response.
#[derive(Debug, Clone, Serialize)]
pub struct Sync {
    /// The batch token to supply in the since param of the next /sync request.
    next_batch: String,
    /// The updates to the presence status of other users.
    presence: Events<PresenceEvent>,
    /// Updates to rooms.
    rooms: Rooms,
}

/// A State Ordering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Batch {
    /// The room ordering key.
    pub room_key: i64,
    /// The presence ordering key.
    pub presence_key: i64,
}

impl Batch {
    /// Create a new `Batch`.
    pub fn new(room_key: i64, presence_key: i64) -> Self {
        Self {
            room_key,
            presence_key,
        }
    }
}

impl Display for Batch {
    /// Make a String from a `Batch`.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}_{}", self.room_key, self.presence_key)
    }
}

impl FromStr for Batch {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let values: Vec<&str> = s.split('_').collect();

        if values.len() != 2 {
            return Err(String::from("Wrong number of tokens"));
        }

        let room_key = i64::from_str_radix(values[0], 10).map_err(|err| err.to_string())?;

        let presence_key = i64::from_str_radix(values[1], 10).map_err(|err| err.to_string())?;

        Ok(Self::new(room_key, presence_key))
    }
}

/// A Sync query options.
#[derive(Clone, Debug)]
pub struct SyncOptions {
    /// The ID of a filter created using the filter API or a filter JSON object encoded as a string.
    pub filter: Option<ContentFilter>,
    /// A point in time to continue a sync from.
    pub since: Option<Batch>,
    /// Controls whether to include the full state for all rooms the user is a member of.
    pub full_state: bool,
    /// Controls whether the client is automatically marked as online by polling this API.
    pub set_presence: Option<PresenceState>,
    /// The maximum time to poll in milliseconds before returning this request.
    pub timeout: u64,
}

/// Sync update context
#[derive(Debug)]
pub enum Context<'a> {
    /// full state
    FullState(&'a Batch),
    /// incremental
    Incremental(&'a Batch),
    /// initial
    Initial,
}

impl Sync {
    /// Query sync.
    pub fn sync(
        connection: &PgConnection,
        homeserver_domain: &str,
        user: &User,
        options: SyncOptions,
    ) -> Result<Self, ApiError> {
        let mut context = Context::Initial;

        if let Some(ref batch) = options.since {
            context = if options.full_state {
                Context::FullState(batch)
            } else {
                Context::Incremental(batch)
            }
        }

        let filter_room = match options.filter {
            Some(filter) => filter.room,
            None => None,
        };

        let (presence_key, presence) = Self::get_presence_events(
            connection,
            homeserver_domain,
            user,
            options.set_presence,
            &context,
        )?;

        let (room_key, rooms) = Self::get_rooms_events(connection, user, filter_room, &context)?;
        let batch = Batch::new(room_key, presence_key);
        let state = Self {
            next_batch: batch.to_string(),
            presence: Events { events: presence },
            rooms,
        };

        Ok(state)
    }

    /// Return presence events for sync from database and options.
    fn get_presence_events(
        connection: &PgConnection,
        homeserver_domain: &str,
        user: &User,
        set_presence: Option<PresenceState>,
        context: &Context<'_>,
    ) -> Result<(i64, Vec<PresenceEvent>), ApiError> {
        let set_presence = match set_presence {
            Some(set_presence) => set_presence,
            None => PresenceState::Online,
        };

        PresenceStatus::upsert(
            connection,
            homeserver_domain,
            &user.id,
            Some(set_presence),
            None,
        )?;

        let since = match *context {
            Context::Incremental(batch) | Context::FullState(batch) => Some(batch.presence_key),
            Context::Initial => None,
        };

        PresenceList::find_events_by_uid(connection, &user.id, since)
    }

    /// Return rooms for sync from database and options.
    fn get_rooms_events(
        connection: &PgConnection,
        user: &User,
        room_filter: Option<RoomFilter>,
        context: &Context<'_>,
    ) -> Result<(i64, Rooms), ApiError> {
        let mut join = HashMap::new();
        let mut invite = HashMap::new();
        let mut leave = HashMap::new();

        let room_memberships = RoomMembership::find_all_by_uid(connection, &user.id)?;

        let mut room_ordering = match *context {
            Context::Incremental(batch) | Context::FullState(batch) => batch.room_key,
            Context::Initial => 0,
        };

        let (is_full_state, since) = match *context {
            Context::Incremental(batch) => (false, batch.room_key),
            Context::FullState(batch) => (true, batch.room_key),
            Context::Initial => (false, -1),
        };

        let (timeline_filter, include_leave) = match room_filter {
            Some(filter) => (filter.timeline, filter.include_leave),
            None => (None, false),
        };

        for room_membership in room_memberships {
            match room_membership.membership.as_str() {
                "join" => {
                    let events: Vec<Event> =
                        Event::find_room_events(connection, &room_membership.room_id, since)?;

                    let room_state_events: Vec<Event> = if is_full_state {
                        Event::get_room_full_state(connection, &room_membership.room_id)?
                    } else {
                        Event::get_room_state_events_since(
                            connection,
                            &room_membership.room_id,
                            since,
                        )?
                    };

                    if events.is_empty() && room_state_events.is_empty() {
                        continue;
                    }

                    let (ordering, timeline) =
                        Self::convert_events_to_timeline(events, &timeline_filter)?;
                    room_ordering = cmp::max(ordering, room_ordering);

                    let state_events: Vec<StateEvent> = room_state_events
                        .iter()
                        .cloned()
                        .map(|e| e.try_into())
                        .collect::<Result<Vec<StateEvent>, ApiError>>()?;

                    join.insert(
                        room_membership.room_id,
                        JoinedRoom {
                            unread_notifications: UnreadNotificationCounts {
                                highlight_count: 0,
                                notification_count: 0,
                            },
                            timeline,
                            state: Events {
                                events: state_events,
                            },
                            account_data: Events { events: Vec::new() },
                            ephemeral: Events { events: Vec::new() },
                        },
                    );
                }
                "invite" => {
                    let room_state_events =
                        Event::get_room_full_state(connection, &room_membership.room_id)?;

                    let state_events: Vec<StrippedState> = room_state_events
                        .iter()
                        .cloned()
                        .map(|e| e.try_into())
                        .collect::<Result<Vec<StrippedState>, ApiError>>()?;

                    invite.insert(
                        room_membership.room_id,
                        InvitedRoom {
                            invite_state: Events {
                                events: state_events,
                            },
                        },
                    );
                }
                "leave" | "ban" => {
                    if !include_leave {
                        continue;
                    }

                    let last_event = Event::find(connection, &room_membership.event_id)?
                        .expect("A room membership should be associated with an event");

                    let events = Event::find_room_events_until(
                        connection,
                        &room_membership.room_id,
                        last_event.ordering,
                    )?;

                    let (ordering, timeline) =
                        Self::convert_events_to_timeline(events, &timeline_filter)?;
                    room_ordering = cmp::max(ordering, room_ordering);

                    let room_state_events = Event::get_room_state_events_until(
                        connection,
                        &room_membership.room_id,
                        &last_event,
                    )?;
                    let state_events: Vec<StateEvent> = room_state_events
                        .iter()
                        .cloned()
                        .map(|e| e.try_into())
                        .collect::<Result<Vec<StateEvent>, ApiError>>()?;

                    leave.insert(
                        room_membership.room_id,
                        LeftRoom {
                            timeline,
                            state: Events {
                                events: state_events,
                            },
                        },
                    );
                }
                _ => (),
            }
        }

        Ok((
            room_ordering,
            Rooms {
                join,
                leave,
                invite,
            },
        ))
    }

    /// Converting events in the correct format for timeline.
    ///
    /// Also returns the max ordering from the given events that will be used
    /// as the `next_batch` token.
    fn convert_events_to_timeline(
        events: Vec<Event>,
        timeline_filter: &Option<RoomEventFilter>,
    ) -> Result<(i64, Timeline), ApiError> {
        let mut room_ordering = 0;
        let mut timeline_events = Vec::new();
        let mut limited = false;

        let length = events.len();

        let count = match *timeline_filter {
            None => 0,
            Some(ref filter) => match filter.limit {
                0 => 0,
                x => {
                    if length > x {
                        limited = true;
                        length - x
                    } else {
                        0
                    }
                }
            },
        };

        for event in events.into_iter().skip(count) {
            room_ordering = cmp::max(room_ordering, event.ordering);

            let value = match EventType::from(event.event_type.as_ref()) {
                EventType::CallAnswer => RoomEvent::CallAnswer(event.try_into()?),
                EventType::CallCandidates => RoomEvent::CallCandidates(event.try_into()?),
                EventType::CallHangup => RoomEvent::CallHangup(event.try_into()?),
                EventType::CallInvite => RoomEvent::CallInvite(event.try_into()?),
                EventType::RoomAliases => RoomEvent::RoomAliases(event.try_into()?),
                EventType::RoomAvatar => RoomEvent::RoomAvatar(event.try_into()?),
                EventType::RoomCanonicalAlias => RoomEvent::RoomCanonicalAlias(event.try_into()?),
                EventType::RoomCreate => RoomEvent::RoomCreate(event.try_into()?),
                EventType::RoomGuestAccess => RoomEvent::RoomGuestAccess(event.try_into()?),
                EventType::RoomHistoryVisibility => {
                    RoomEvent::RoomHistoryVisibility(event.try_into()?)
                }
                EventType::RoomJoinRules => RoomEvent::RoomJoinRules(event.try_into()?),
                EventType::RoomMember => RoomEvent::RoomMember(event.try_into()?),
                EventType::RoomMessage => RoomEvent::RoomMessage(event.try_into()?),
                EventType::RoomName => RoomEvent::RoomName(event.try_into()?),
                EventType::RoomPowerLevels => RoomEvent::RoomPowerLevels(event.try_into()?),
                EventType::RoomThirdPartyInvite => {
                    RoomEvent::RoomThirdPartyInvite(event.try_into()?)
                }
                EventType::RoomTopic => RoomEvent::RoomTopic(event.try_into()?),
                _ => {
                    println!("unhandled {:?}", event.event_type);
                    continue;
                }
            };

            timeline_events.push(value);
        }

        Ok((
            room_ordering,
            Timeline {
                events: timeline_events,
                limited,
                prev_batch: String::from(""),
            },
        ))
    }
}

#[test]
fn batch_to_str() {
    let batch = Batch::new(10, 10);
    assert_eq!(batch.to_string(), String::from("10_10"));
}

#[test]
fn batch_parse() {
    let batch = Batch::from_str("10_12").unwrap();
    assert_eq!(batch.room_key, 10);
    assert_eq!(batch.presence_key, 12);
}

#[test]
fn batch_parse_non_number() {
    let batch = Batch::from_str("10_12a");
    assert!(batch.is_err());
}

#[test]
fn batch_parse_too_many() {
    let batch = Batch::from_str("10_12_12");
    assert!(batch.is_err());
}
