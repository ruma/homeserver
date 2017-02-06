//! Matrix rooms.

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use diesel::{Connection, ExecuteDsl, ExpressionMethods, FilterDsl, FindDsl, LoadDsl, OrderDsl, insert};
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::Error as DieselError;
use ruma_events::EventType;
use ruma_events::stripped::StrippedState;
use ruma_events::room::avatar::AvatarEvent;
use ruma_events::room::canonical_alias::{CanonicalAliasEvent, CanonicalAliasEventContent};
use ruma_events::room::create::{CreateEvent, CreateEventContent};
use ruma_events::room::history_visibility::{
    HistoryVisibility,
    HistoryVisibilityEvent,
    HistoryVisibilityEventContent,
};
use ruma_events::room::join_rules::{
    JoinRule,
    JoinRulesEvent,
    JoinRulesEventContent,
};
use ruma_events::room::name::{NameEvent, NameEventContent};
use ruma_events::room::power_levels::{PowerLevelsEvent, PowerLevelsEventContent};
use ruma_events::room::topic::{TopicEvent, TopicEventContent};
use ruma_identifiers::{EventId, RoomAliasId, RoomId, UserId};

use error::ApiError;
use models::event::{Event, NewEvent};
use models::room_alias::{NewRoomAlias, RoomAlias};
use models::room_membership::RoomMembership;
use schema::{events, rooms};

/// Options provided by the user to customize the room upon creation.
pub struct CreationOptions {
    /// An initial alias for the room.
    pub alias: Option<String>,
    /// Whether or not the room should be federated.
    pub federate: bool,
    /// A list of state events to set in the new room.
    pub initial_state: Option<Vec<Box<StrippedState>>>,
    /// A list of users to invite to the room.
    pub invite_list: Option<Vec<UserId>>,
    /// An initial name for the room.
    pub name: Option<String>,
    /// A convenience parameter for setting a few default state events.
    pub preset: RoomPreset,
    /// An initial topic for the room.
    pub topic: Option<String>,
}

/// A new Matrix room, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "rooms"]
pub struct NewRoom {
    /// The room's unique ID.
    pub id: RoomId,
    /// The ID of the user creating the room.
    pub user_id: UserId,
    /// Whether or not the room is visible in the directory.
    pub public: bool,
}

/// A Matrix room.
#[derive(Debug, Queryable)]
pub struct Room {
    /// The room's unique ID.
    pub id: RoomId,
    /// The ID of the user who created the room.
    pub user_id: UserId,
    /// Whether or not the room is visible in the directory.
    pub public: bool,
    /// The time the room was created.
    pub created_at: PgTimestamp,
}

/// A convenience parameter for setting a few default state events.
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum RoomPreset {
    /// `join_rules` is set to `invite` and `history_visibility` is set to `shared`.
    #[serde(rename="private_chat")]
    PrivateChat,
    /// `join_rules` is set to `public` and `history_visibility` is set to `shared`.
    #[serde(rename="public_chat")]
    PublicChat,
    /// Same as `PrivateChat`, but all initial invitees get the same power level as the creator.
    #[serde(rename="trusted_private_chat")]
    TrustedPrivateChat,
}

/// Indicates whether or not that the room will be shown in the published room list.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum RoomVisibility {
    /// The room will be private.
    #[serde(rename="private")]
    Private,
    /// The room will be public.
    #[serde(rename="public")]
    Public,
}

impl Room {
    /// Creates a new room in the database.
    ///
    /// The creation order of the events should be the following:
    /// 1. Events set by presets.
    /// 2. Events listed in initial_state, in the order that they are listed.
    /// 3. Events implied by name and topic.
    /// 4. Invite events implied by invite and invite_3pid.
    pub fn create(
        connection: &PgConnection,
        new_room: &NewRoom,
        homeserver_domain: &str,
        creation_options: &CreationOptions,
    ) -> Result<Room, ApiError> {
        connection.transaction::<Room, ApiError, _>(|| {
            let room: Room = insert(new_room)
                .into(rooms::table)
                .get_result(connection)
                .map_err(ApiError::from)?;

            let mut new_events = Vec::new();

            let new_create_event: NewEvent = CreateEvent {
                content: CreateEventContent {
                    creator: new_room.user_id.clone(),
                    federate: creation_options.federate,
                },
                event_id: EventId::new(homeserver_domain)?,
                event_type: EventType::RoomCreate,
                prev_content: None,
                room_id: room.id.clone(),
                state_key: "".to_string(),
                unsigned: None,
                user_id: new_room.user_id.clone(),
            }.try_into()?;

            new_events.push(new_create_event);

            let mut is_canonical_alias_set = false;
            let mut is_history_visibility_set = false;
            let mut is_power_levels_set = false;
            let mut is_trusted_private_chat = false;
            let mut new_room_aliases = Vec::new();

            match creation_options.preset {
                RoomPreset::PrivateChat => {
                    let new_join_rules_event: NewEvent = JoinRulesEvent {
                        content: JoinRulesEventContent { join_rule: JoinRule::Invite },
                        event_id: EventId::new(homeserver_domain)?,
                        event_type: EventType::RoomJoinRules,
                        prev_content: None,
                        room_id: room.id.clone(),
                        state_key: "".to_string(),
                        unsigned: None,
                        user_id: new_room.user_id.clone(),
                    }.try_into()?;

                    new_events.push(new_join_rules_event);
                },
                RoomPreset::PublicChat => {
                    let new_join_rules_event: NewEvent = JoinRulesEvent {
                        content: JoinRulesEventContent { join_rule: JoinRule::Public },
                        event_id: EventId::new(homeserver_domain)?,
                        event_type: EventType::RoomJoinRules,
                        prev_content: None,
                        room_id: room.id.clone(),
                        state_key: "".to_string(),
                        unsigned: None,
                        user_id: new_room.user_id.clone(),
                    }.try_into()?;

                    new_events.push(new_join_rules_event);
                },
                RoomPreset::TrustedPrivateChat => {
                    is_trusted_private_chat = true;

                    let new_join_rules_event: NewEvent = JoinRulesEvent {
                        content: JoinRulesEventContent { join_rule: JoinRule::Invite },
                        event_id: EventId::new(homeserver_domain)?,
                        event_type: EventType::RoomJoinRules,
                        prev_content: None,
                        room_id: room.id.clone(),
                        state_key: "".to_string(),
                        unsigned: None,
                        user_id: new_room.user_id.clone(),
                    }.try_into()?;

                    new_events.push(new_join_rules_event);
                }
            }

            if let Some(ref name) = creation_options.name {
                let new_name_event: NewEvent = NameEvent {
                    content: NameEventContent {
                        name: name.to_string(),
                    },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomName,
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: new_room.user_id.clone(),
                }.try_into()?;

                new_events.push(new_name_event);
            }

            if let Some(ref topic) = creation_options.topic {
                let new_topic_event: NewEvent = TopicEvent {
                    content: TopicEventContent {
                        topic: topic.to_string(),
                    },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomTopic,
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: new_room.user_id.clone(),
                }.try_into()?;

                new_events.push(new_topic_event);
            }

            if creation_options.initial_state.is_some() {
                let initial_events = creation_options.initial_state.clone().unwrap();

                for state_event in initial_events {
                    match *state_event {
                        StrippedState::RoomAliases(event) => {
                            for alias in event.content.aliases {
                                if alias.hostname().to_string() != homeserver_domain {
                                    return Err(
                                        ApiError::unimplemented("Federation is not yet supported".to_string())
                                    );
                                }

                                let new_room_alias = NewRoomAlias {
                                    alias: alias,
                                    room_id: room.id.clone(),
                                    user_id: new_room.user_id.clone(),
                                    servers: vec![homeserver_domain.to_string()],
                                };

                                new_room_aliases.push(new_room_alias);
                            }
                        },
                        StrippedState::RoomAvatar(event) => {
                            let new_avatar_event: NewEvent = AvatarEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomAvatar,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_avatar_event);
                        },
                        StrippedState::RoomCanonicalAlias(event) => {
                            if event.content.alias.hostname().to_string() != homeserver_domain {
                                return Err(ApiError::unimplemented("Federation is not yet supported".to_string()));
                            }

                            is_canonical_alias_set = true;

                            let new_canonical_alias_event: NewEvent = CanonicalAliasEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomCanonicalAlias,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_canonical_alias_event);
                        },
                        StrippedState::RoomGuestAccess(_) => {
                            Err(ApiError::unimplemented("Guests are not yet supported".to_string()))?
                        },
                        StrippedState::RoomHistoryVisibility(event) => {
                            is_history_visibility_set = true;

                            let new_history_visibility_event: NewEvent = HistoryVisibilityEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomHistoryVisibility,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_history_visibility_event);
                        },
                        StrippedState::RoomJoinRules(event) => {
                            let new_join_rules_event: NewEvent = JoinRulesEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomJoinRules,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_join_rules_event);
                        },
                        StrippedState::RoomName(event) => {
                            if creation_options.name.is_some() {
                                continue;
                            }

                            let new_name_event: NewEvent = NameEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomName,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_name_event);
                        },
                        StrippedState::RoomPowerLevels(mut event) => {
                            is_power_levels_set = true;
                            event.content.users.insert(room.user_id.clone(), 100);

                            if is_trusted_private_chat && creation_options.invite_list.is_some() {
                                for user in creation_options.invite_list.clone().unwrap() {
                                    event.content.users.insert(user.clone(), 100);
                                }
                            }

                            let new_power_levels_event: NewEvent = PowerLevelsEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomPowerLevels,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_power_levels_event);
                        },
                        StrippedState::RoomThirdPartyInvite(_) => {
                            Err(ApiError::unimplemented("Third party invites are not yet supported".to_string()))?
                        },
                        StrippedState::RoomTopic(event) => {
                            if creation_options.topic.is_some() {
                                continue;
                            }

                            let new_topic_event: NewEvent = TopicEvent {
                                content: event.content.clone(),
                                event_id: EventId::new(homeserver_domain)?,
                                event_type: EventType::RoomTopic,
                                prev_content: None,
                                room_id: room.id.clone(),
                                state_key: event.state_key.to_string(),
                                unsigned: None,
                                user_id: room.user_id.clone(),
                            }.try_into()?;

                            new_events.push(new_topic_event);
                        }
                        StrippedState::RoomCreate(_) | StrippedState::RoomMember(_) => {
                            Err(
                                ApiError::bad_json(
                                    "m.room.create and m.room.member are not supported by 'initial_state'".to_string()
                                )
                            )?
                        }
                    }
                }
            }

            if !is_history_visibility_set {
                let new_history_visibility_event: NewEvent = HistoryVisibilityEvent {
                    content: HistoryVisibilityEventContent {
                        history_visibility: HistoryVisibility::Shared,
                    },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomHistoryVisibility,
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: new_room.user_id.clone(),
                }.try_into()?;

                new_events.push(new_history_visibility_event);
            }

            if !is_power_levels_set {
                let mut user_power = HashMap::<UserId, u64>::new();
                user_power.insert(room.user_id.clone(), 100);

                if is_trusted_private_chat && creation_options.invite_list.is_some() {
                    for user in creation_options.invite_list.clone().unwrap() {
                        user_power.insert(user.clone(), 100);
                    }
                }

                let new_power_levels_event: NewEvent = PowerLevelsEvent {
                    content: PowerLevelsEventContent {
                        ban: 50,
                        events: HashMap::new(),
                        events_default: 0,
                        invite: 50,
                        kick: 50,
                        redact: 50,
                        state_default: 0,
                        users: user_power,
                        users_default: 0,
                    },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomPowerLevels,
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: room.user_id.clone(),
                }.try_into()?;

                new_events.push(new_power_levels_event);
            }

            if creation_options.alias.is_some() && !is_canonical_alias_set {
                let new_canonical_alias_event: NewEvent = CanonicalAliasEvent {
                    content: CanonicalAliasEventContent {
                        alias: RoomAliasId::try_from(
                            &format!("#{}:{}", creation_options.alias.clone().unwrap(), homeserver_domain)
                        )?
                    },
                    event_id: EventId::new(homeserver_domain)?,
                    event_type: EventType::RoomCanonicalAlias,
                    prev_content: None,
                    room_id: room.id.clone(),
                    state_key: "".to_string(),
                    unsigned: None,
                    user_id: room.user_id.clone(),
                }.try_into()?;

                new_events.push(new_canonical_alias_event);
            }

            insert(&new_events)
                .into(events::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            for alias in new_room_aliases {
                RoomAlias::create(connection, homeserver_domain, &alias)?;
            }

            if let Some(ref alias) = creation_options.alias {
                let new_room_alias = NewRoomAlias {
                    alias: RoomAliasId::try_from(&format!("#{}:{}", alias, homeserver_domain))?,
                    room_id: room.id.clone(),
                    user_id: new_room.user_id.clone(),
                    servers: vec![homeserver_domain.to_string()],
                };

                RoomAlias::create(connection, homeserver_domain, &new_room_alias)?;
            }

            if let Some(ref invite_list) = creation_options.invite_list {
                RoomMembership::create_memberships(connection, &room, invite_list, homeserver_domain)?;
            }

            Ok(room)
        }).map_err(ApiError::from)
    }

    /// Looks up the most recent power levels event for the room.
    ///
    /// If the room does not have a power levels event, a default one is created according to the
    /// specification.
    pub fn current_power_levels(&self, connection: &PgConnection)
    -> Result<PowerLevelsEventContent, ApiError> {
        match events::table
            .filter(events::room_id.eq(self.id.clone()))
            .filter(events::event_type.eq(EventType::RoomPowerLevels.to_string()))
            .order(events::ordering.desc())
            .first::<Event>(connection)
        {
            Ok(event) => {
                let power_levels_event: PowerLevelsEvent = event.try_into()?;

                Ok(power_levels_event.content)
            }
            Err(error) => match error {
                DieselError::NotFound => Ok(PowerLevelsEventContent {
                    ban: 50,
                    events: HashMap::new(),
                    events_default: 0,
                    invite: 50,
                    kick: 50,
                    redact: 50,
                    state_default: 0,
                    users: HashMap::new(),
                    users_default: 0,
                }),
                _ => Err(error.into()),
            },
        }
    }

    /// Look up a `Room` given the `RoomId`.
    pub fn find(connection: &PgConnection, room_id: &RoomId)
    -> Result<Option<Room>, ApiError> {
        let result = rooms::table
            .find(room_id)
            .get_result(connection);

        match result {
            Ok(room) => Ok(Some(room)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }
}
