//! Matrix sync.

use std::cmp;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::i64;
use std::iter::Iterator;
use std::str::FromStr;

use diesel::pg::PgConnection;
use ruma_events::room::member::MemberEvent;
use ruma_events::room::message::MessageEvent;
use ruma_events::room::history_visibility::HistoryVisibilityEvent;
use ruma_events::presence::PresenceEvent;
use ruma_events::presence::PresenceState;
use ruma_events::collections::all::{RoomEvent, StateEvent};
use ruma_identifiers::RoomId;
use serde_json::Value;
use time;

use error::ApiError;
use models::event::Event;
use models::filter::{ContentFilter, RoomEventFilter, RoomFilter};
use models::room_membership::RoomMembership;
use models::presence_list::PresenceList;
use models::presence_status::PresenceStatus;
use models::user::User;

#[derive(Debug, Clone, Serialize)]
struct UnreadNotificationCounts {
    highlight_count: u64,
    notification_count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct Timeline {
    limited: bool,
    prev_batch: String,
    events: Vec<RoomEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct Events<T> {
    events: Vec<T>,
}

#[derive(Debug, Clone, Serialize)]
struct LeftRoom {
    timeline: Timeline,
    state: Events<StateEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct InvitedRoom {
    invite_state: Events<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct JoinedRoom {
    unread_notifications: UnreadNotificationCounts,
    timeline: Timeline,
    state: Events<StateEvent>,
    account_data: Events<Value>,
    ephemeral: Events<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct Rooms {
    join: HashMap<RoomId, JoinedRoom>,
    leave: HashMap<RoomId, LeftRoom>,
    invite: HashMap<RoomId, InvitedRoom>,
}

/// A Sync response.
#[derive(Debug, Clone, Serialize)]
pub struct Sync {
    next_batch: String,
    presence: Events<PresenceEvent>,
    rooms: Rooms,
}

/// A State Ordering.
#[derive(Debug, Clone)]
pub struct Batch {
    /// The room ordering key.
    pub room_key: i64,
    /// The presence ordering key.
    pub presence_key: i64,
}

impl Batch {
    /// Create a new `Batch`.
    pub fn new(room_key: i64, presence_key: i64) -> Batch {
        Batch {
            room_key: room_key,
            presence_key: presence_key,
        }
    }
}

impl Display for Batch {
    /// Make a String from a `Batch`.
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}_{}", self.room_key, self.presence_key)
    }
}

impl FromStr for Batch {
    type Err = String;
    fn from_str(s: &str) -> Result<Batch, String> {
        let values: Vec<&str> = s.split('_').collect();

        if values.len() != 2 {
            return Err(String::from("Wrong number of tokens"));
        }

        let room_key = i64::from_str_radix(values[0], 10)
            .map_err(|err| err.to_string())?;

        let presence_key = i64::from_str_radix(values[1], 10)
            .map_err(|err| err.to_string())?;

        Ok(Batch::new(room_key, presence_key))
    }
}

/// A Sync query options.
#[derive(Debug)]
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
        options: SyncOptions
    ) -> Result<Sync, ApiError> {
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
            None => None
        };
        let (presence_key, presence) = Sync::get_presence_events(
            connection,
            homeserver_domain,
            user,
            options.set_presence,
            &context
        )?;

        let (room_key, rooms) = Sync::get_rooms_events(connection, user, filter_room, &context)?;
        let batch = Batch::new(room_key, presence_key);
        let state = Sync {
            next_batch: batch.to_string(),
            presence: Events {
                events: presence,
            },
            rooms: rooms,
        };
        Ok(state)
    }

    /// Return presence events for sync from database and options.
    fn get_presence_events(
        connection: &PgConnection,
        homeserver_domain: &str,
        user: &User,
        set_presence: Option<PresenceState>,
        context: &Context
    ) -> Result<(i64, Vec<PresenceEvent>), ApiError> {
        let set_presence = match set_presence {
            Some(set_presence) => set_presence,
            None => PresenceState::Online,
        };
        PresenceStatus::upsert(connection, homeserver_domain, &user.id, Some(set_presence), None)?;

        let since = match *context {
            Context::Incremental(batch) | Context::FullState(batch)  => {
                Some(time::Timespec::new(batch.presence_key, 0))
            }
            Context::Initial => None,
        };

        PresenceList::find_events_by_uid(
            connection,
            &user.id,
            since
        )
    }

    /// Return rooms for sync from database and options.
    fn get_rooms_events(
        connection: &PgConnection,
        user: &User,
        room_filter: Option<RoomFilter>,
        context: &Context,
    ) -> Result<(i64, Rooms), ApiError> {
        let mut room_ordering = 0;
        let mut join = HashMap::new();
        let mut invite = HashMap::new();
        let mut leave = HashMap::new();

        let room_memberships = RoomMembership::find_by_user_id_order_by_room_id(connection, &user.id)?;

        let since = match *context {
            Context::Incremental(batch) => batch.room_key,
            Context::FullState(_) | Context::Initial => -1,
        };
        let timeline_filter = match room_filter {
            Some(filter) => filter.timeline,
            None => None,
        };

        for room_membership in room_memberships {
            let events: Vec<Event> = Event::find_room_events(connection, &room_membership.room_id, since)?;
            if events.is_empty() {
                continue;
            }
            match room_membership.membership.as_str() {
                "join" => {
                    let (i, timeline) = Sync::convert_events_to_timeline(events, &timeline_filter)?;
                    room_ordering = cmp::max(i, room_ordering);
                    join.insert(room_membership.room_id, JoinedRoom {
                        unread_notifications: UnreadNotificationCounts {
                            highlight_count: 0,
                            notification_count: 0,
                        },
                        timeline: timeline,
                        state: Events {
                            events: Vec::new(),
                        },
                        account_data: Events {
                            events: Vec::new(),
                        },
                        ephemeral: Events {
                            events: Vec::new(),
                        },
                    });
                },
                "invite" => {
                    invite.insert(room_membership.room_id, InvitedRoom {
                        invite_state: Events {
                            events: Vec::new(),
                        },
                    });
                },
                "leave" => {
                    let (i, timeline) = Sync::convert_events_to_timeline(events, &timeline_filter)?;
                    room_ordering = cmp::max(i, room_ordering);
                    leave.insert(room_membership.room_id, LeftRoom {
                        timeline: timeline,
                        state: Events {
                            events: Vec::new(),
                        },
                    });
                },
                _ => (),
            }
        }

        Ok((room_ordering, Rooms {
            join: join,
            leave: leave,
            invite: invite,
        }))
    }

    /// Converting events in the correct format for timeline.
    fn convert_events_to_timeline(
        events: Vec<Event>,
        timeline_filter: &Option<RoomEventFilter>
    ) -> Result<(i64, Timeline), ApiError> {
        let mut room_ordering = 0;
        let mut timeline_events = Vec::new();
        let mut limited = false;

        let length = events.len();

        let count = match *timeline_filter {
            None => 0,
            Some(ref filter) => {
                match filter.limit {
                    0 => 0,
                    x => {
                        if length > x {
                            limited = true;
                            length - x
                        } else {
                            0
                        }
                    },
                }
            }
        };

        for event in events.into_iter().skip(count) {
            room_ordering = cmp::max(room_ordering, event.ordering);
            let value = match event.event_type.as_ref() {
                "m.room.member" => {
                    let member_event: MemberEvent = event.try_into()?;
                    RoomEvent::RoomMember(member_event)
                },
                "m.room.message" => {
                    let message_event: MessageEvent = event.try_into()?;
                    RoomEvent::RoomMessage(message_event)
                },
                "m.room.history_visibility" => {
                    let history_visibility_event: HistoryVisibilityEvent = event.try_into()?;
                    RoomEvent::RoomHistoryVisibility(history_visibility_event)
                },
                _ => {
                    println!("unhandled {:?}", event.event_type);
                    continue;
                },
            };
            timeline_events.push(value);
        }

        Ok((room_ordering, Timeline {
            events: timeline_events,
            limited: limited,
            prev_batch: String::from(""),
        }))
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
