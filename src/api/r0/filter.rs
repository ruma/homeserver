//! Endpoints for filter rooms.
use bodyparser;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;
use ruma_identifiers::{RoomId, UserId};
use serde_json::de::from_str;
use serde_json::value::ToJson;

use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, FilterIdParam, JsonRequest, MiddlewareChain, UserIdParam};
use models::filter::{Filter as DataFilter};
use models::user::User;
use modifier::SerializableResponse;

/// Filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    /// A list of event types to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub not_types: Vec<String>,
    /// The maximum number of events to return.
    pub limit: usize,
    /// A list of senders IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub senders: Vec<UserId>,
    /// A list of event types to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub types: Vec<String>,
    /// A list of sender IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub not_senders: Vec<UserId>,
}

/// RoomEventFilter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEventFilter {
    /// A list of event types to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub not_types: Vec<String>,
    /// A list of event types to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub types: Vec<String>,
    /// A list of room IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub not_rooms: Vec<RoomId>,
    /// A list of room IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub rooms: Vec<RoomId>,
    /// The maximum number of events to return.
    pub limit: usize,
    /// A list of sender IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub not_senders: Vec<UserId>,
    /// A list of senders IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_user_id")]
    pub senders: Vec<UserId>,
}

fn default_include_leave() -> bool {
    false
}

fn default_vec_room_id() -> Vec<RoomId> {
    Vec::new()
}

fn default_vec_user_id() -> Vec<UserId> {
    Vec::new()
}

fn is_false(test: &bool) -> bool {
    !test
}


/// RoomFilter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFilter {
    /// Include rooms that the user has left in the sync, default false
    #[serde(default = "default_include_leave")]
    #[serde(skip_serializing_if = "is_false")]
    pub include_leave: bool,
    /// The per user account data to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_data: Option<RoomEventFilter>,
    /// The message and state update events to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<RoomEventFilter>,
    /// The events that aren't recorded in the room history, e.g. typing and receipts, to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<RoomEventFilter>,
    /// The state events to include for rooms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<RoomEventFilter>,
    /// A list of room IDs to exclude.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub not_rooms: Vec<RoomId>,
    /// A list of room IDs to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_vec_room_id")]
    pub rooms: Vec<RoomId>,
}

/// EventFormat
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventFormat {
    /// 'client' will return the events in a format suitable for clients.
    Client,
    /// 'federation' will return the raw event as receieved over federation.
    Federation,
}

impl ::serde::Serialize for EventFormat {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: ::serde::Serializer,
    {
        // Serialize the enum as a string.
        serializer.serialize_str(match *self {
            EventFormat::Client => "client",
            EventFormat::Federation => "federation",
        })
    }
}

impl ::serde::Deserialize for EventFormat {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: ::serde::Deserializer,
    {
        struct Visitor;

        impl ::serde::de::Visitor for Visitor {
            type Value = EventFormat;

            fn visit_str<E>(&mut self, value: &str) -> Result<EventFormat, E>
                where E: ::serde::de::Error,
            {
                match value {
                    "client" => Ok(EventFormat::Client),
                    "federation" => Ok(EventFormat::Federation),
                    _ => Err(E::invalid_value(&format!("unknown {} variant: {}",
                                                       stringify!( EventFormat), value))),
                }
            }
        }

        // Deserialize the enum from a string.
        deserializer.deserialize_str(Visitor)
    }
}

/// FilterResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResponse {
    /// Filters to be applied to room data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<RoomFilter>,
    /// The presence updates to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Filter>,
    /// The user account data that isn't associated with rooms to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_data: Option<Filter>,
    /// The format to use for events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_format: Option<EventFormat>,
    /// List of event fields to include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub event_fields: Vec<String>,
}

/// The GET `/user/:user_id/filter/:filter_id` endpoint.
pub struct GetFilter;

middleware_chain!(GetFilter, [AccessTokenAuth, FilterIdParam, UserIdParam]);

impl Handler for GetFilter {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let filter_id = request.extensions.get::<FilterIdParam>()
            .expect("FilterIdParam should ensure a FilterIdParam").clone();

        let connection = DB::from_request(request)?;
        let filter = DataFilter::find(&connection, user_id, filter_id)?;
        let response: FilterResponse = from_str(&filter.content).map_err(ApiError::from)?;
        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The POST `/user/:user_id/filter` endpoint.
pub struct PostFilter;

#[derive(Debug, Serialize)]
struct PostFilterResponse {
    filter_id: String,
}

middleware_chain!(PostFilter, [JsonRequest, AccessTokenAuth, UserIdParam]);

impl Handler for PostFilter {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        if user_id != user.id {
            Err(ApiError::unauthorized("The given user_id does not correspond to the authenticated user".to_string()))?;
        }

        let filter = match request.get::<bodyparser::Struct<FilterResponse>>() {
            Ok(Some(account_password_request)) => account_password_request,
            Ok(None) | Err(_) => {
                let error = ApiError::bad_json(None);
                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;

        let id = DataFilter::create(&connection, user_id, filter. to_json().to_string())?;

        let response = PostFilterResponse {
            filter_id: id.to_string(),
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn basic_test() {
        let test = Test::new();
        let access_token = test.create_access_token_with_username("carl");
        let user_id = "@carl:ruma.test";

        let filter_id = test.create_filter(&access_token, user_id, r#"{"room":{"timeline":{"limit":10}}}"#);

        let get_filter_path = format!(
            "/_matrix/client/r0/user/{}/filter/{}?access_token={}",
            user_id,
            filter_id,
            access_token
        );

        let response = test.get(&get_filter_path);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, r#"{"room":{"timeline":{"limit":10}}}"#);
    }

    #[test]
    fn invalid_user() {
        let test = Test::new();
        let _ = test.create_access_token_with_username("carl");
        let alice = test.create_access_token_with_username("alice");
        let user_id = "@carl:ruma.test";
        let filter_path = format!(
            "/_matrix/client/r0/user/{}/filter?access_token={}",
            user_id,
            alice
        );

        let response = test.post(&filter_path, r#"{"room":{"timeline":{"limit":10}}}"#);
        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn get_not_found() {
        let test = Test::new();
        let access_token = test.create_access_token_with_username("carl");
        let user_id = "@carl:ruma.test";

        let get_filter_path = format!(
            "/_matrix/client/r0/user/{}/filter/{}?access_token={}",
            user_id,
            1,
            access_token
        );

        let response = test.get(&get_filter_path);
        assert_eq!(response.status, Status::NotFound);
    }
}
