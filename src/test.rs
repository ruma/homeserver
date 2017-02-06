use std::sync::{ONCE_INIT, Once};
use std::convert::TryFrom;

use env_logger;
use diesel::Connection;
use diesel::migrations::setup_database;
use diesel::pg::PgConnection;
use iron;
use iron::headers::{ContentType, Headers};
use iron::method::Method;
use iron::status::Status;
use iron_test::{request, response};
use mount::Mount;
use r2d2::{Config as R2D2Config, CustomizeConnection};
use r2d2_diesel::Error as R2D2DieselError;
use serde_json::{Value, from_str, to_string};
use ruma_events::presence::PresenceState;
use ruma_identifiers::UserId;

use config::Config;
use embedded_migrations::run as run_pending_migrations;
use query::{SyncOptions, Batch};
use server::Server;

static START: Once = ONCE_INIT;

const DATABASE_URL: &'static str = "postgres://postgres:test@postgres:5432/ruma_test";
const POSTGRES_URL: &'static str = "postgres://postgres:test@postgres:5432";

/// Used to return the randomly generated user id and access token
#[derive(Debug)]
pub struct TestUser {
    pub id: String,
    pub token: String,
    pub name: String,
}

impl TestUser {
    pub fn new(user: UserId, token: String) -> Self {
        TestUser { id: user.to_string(), token: token, name: user.localpart().to_string() }
    }
}

/// Manages the Postgres for the duration of a test case and provides helper methods for
/// interacting with the Ruma API server.
pub struct Test {
    mount: Mount,
}

/// An HTTP response from the server.
#[derive(Debug)]
pub struct Response {
    pub body: String,
    pub headers: Headers,
    json: Option<Value>,
    pub status: Status,
}

/// An R2D2 plugin for starting a test transaction whenever a database connection is acquired from
/// the connection pool.
#[derive(Debug)]
pub struct TestTransactionConnectionCustomizer;

impl CustomizeConnection<PgConnection, R2D2DieselError> for TestTransactionConnectionCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), R2D2DieselError> {
        conn.begin_test_transaction().map_err(|error| R2D2DieselError::QueryError(error))
    }
}

impl Test {
    /// Creates a new `Test`.
    pub fn new() -> Self {
        // Since we don't have control of the `main` function during tests, we initialize the
        // logger here. It will only actually initialize on the first test that is run. Subsequent
        // calls will return an error, but we don't care, so just ignore the result.
        match env_logger::init() {
            _ => {}
        }

        START.call_once(|| {
            if PgConnection::establish(DATABASE_URL).is_ok() {
                let connection = PgConnection::establish(POSTGRES_URL).expect(
                    "Failed to connect to Postgres to drop the existing ruma_test table."
                );

                connection.silence_notices(|| {
                    connection.execute("DROP DATABASE IF EXISTS ruma_test").expect(
                        "Failed to drop the existing ruma_test table."
                    );
                });
            }

            let pg_connection = PgConnection::establish(POSTGRES_URL).expect(
                "Failed to connect to Postgres."
            );

            pg_connection.execute("CREATE DATABASE ruma_test").expect(
                "Failed to create the ruma_test table."
            );

            let db_connection = PgConnection::establish(DATABASE_URL).expect(
                "Failed to connect to Postgres database."
            );

            setup_database(&db_connection).expect("Failed to create migrations table.");
            run_pending_migrations(&db_connection).expect("Failed to run migrations.");
        });

        let config = Config {
            bind_address: "127.0.0.1".to_string(),
            bind_port: "0".to_string(),
            domain: "ruma.test".to_string(),
            macaroon_secret_key: "YymznQHmKdN9B4f7iBalJB1tWEDy9LdaFSQJEtB3R5w=".into(),
            postgres_url: DATABASE_URL.to_string(),
        };

        let r2d2_config = R2D2Config::builder()
            .pool_size(1)
            .connection_customizer(Box::new(TestTransactionConnectionCustomizer))
            .build();

        let server = match Server::with_options(&config, r2d2_config, false) {
            Ok(server) => server,
            Err(error) => panic!("Failed to create Iron server: {}", error),
        };

        Test {
            mount: server.into_mount(),
        }
    }

    /// Makes a GET request to the server.
    pub fn get(&self, path: &str) -> Response {
        self.request(Method::Get, path, "")
    }

    /// Makes a POST request to the server.
    pub fn post(&self, path: &str, body: &str) -> Response {
        self.request(Method::Post, path, body)
    }

    /// Makes a DELETE request to the server.
    pub fn delete(&self, path: &str) -> Response {
        self.request(Method::Delete, path, "")
    }

    /// Makes a PUT request to the server.
    pub fn put(&self, path: &str, body: &str) -> Response {
        self.request(Method::Put, path, body)
    }

    /// Makes a request to the server.
    pub fn request(&self, method: Method, path: &str, body: &str) -> Response {
        let mut headers = Headers::new();

        headers.set(ContentType::json());

        let response = match request::request(
            method,
            &format!("http://ruma.test{}", path)[..],
            body,
            headers,
            &self.mount,
        ) {
            Ok(response) => response,
            Err(error) => error.response,
        };

        Response::from_iron_response(response)
    }

    /// Registers a new user account and returns the response of the API call.
    pub fn register_user(&self, body: &str) -> Response {
        self.post("/_matrix/client/r0/register", body)
    }

    /// Registers a new user account with a random user id and returns
    /// the `TestUser`
    pub fn create_user(&self) -> TestUser {
        let response = self.register_user(&format!(r#"{{"password": "secret"}}"#));

        let access_token = response.json().find("access_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        let user_id = response.json().find("user_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        TestUser::new(UserId::try_from(&user_id).unwrap(), access_token)
    }

    /// Creates a room given the body parameters and returns the room ID as a string.
    pub fn create_room_with_params(&self, access_token: &str, body: &str) -> String {
        self.post(&format!("/_matrix/client/r0/createRoom?access_token={}", access_token), body)
            .json()
            .find("room_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Creates a room and returns the room ID as a string.
    pub fn create_room(&self, access_token: &str) -> String {
        self.create_room_with_params(access_token, "{}")
    }

    /// Creates a public room and returns the room ID as a string.
    pub fn create_public_room(&self, access_token: &str) -> String {
        self.create_room_with_params(access_token, r#"{"visibility": "public"}"#)
    }

    /// Creates a private room and returns the room ID as a string.
    pub fn create_private_room(&self, access_token: &str) -> String {
        self.create_room_with_params(access_token, r#"{"visibility": "private"}"#)
    }

    /// Invite a `User` to a `Room`.
    pub fn invite(&self, access_token: &str, room_id: &str, invitee_id: &str) -> Response {
        let body = format!(r#"{{"user_id": "{}"}}"#, invitee_id);
        let path = format!(
            "/_matrix/client/r0/rooms/{}/invite?access_token={}",
            room_id,
            access_token
        );

        self.post(&path, &body)
    }

    /// Look up a `RoomId` using an alias.
    pub fn get_room_by_alias(&self, alias: &str) -> Response {
        self.get(&format!("/_matrix/client/r0/directory/room/{}", alias))
    }

    /// Join an existent room.
    pub fn join_room(&self, access_token: &str, room_id: &str) -> Response {
        let join_path = format!(
            "/_matrix/client/r0/rooms/{}/join?access_token={}",
            room_id,
            access_token
        );

        self.post(&join_path, r"{}")
    }

    /// Create tag
    pub fn create_tag(&self, access_token: &str, room_id: &str, user_id: &str, tag: &str, content: &str) {
        let put_tag_path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/tags/{}?access_token={}",
            user_id,
            room_id,
            tag,
            access_token
        );

        let response = self.put(&put_tag_path, content);
        assert_eq!(response.status, Status::Ok);
    }

    /// Create a filter
    pub fn create_filter(&self, access_token: &str, user_id: &str, content: &str) -> String {
        let filter_path = format!(
            "/_matrix/client/r0/user/{}/filter?access_token={}",
            user_id,
            access_token
        );

        let response = self.post(&filter_path, content);
        assert_eq!(response.status, Status::Ok);
        response
            .json()
            .find("filter_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Send a message to room.
    pub fn send_message(&self, access_token: &str, room_id: &str, message: &str) -> Response {
        let create_event_path = format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/1?access_token={}",
            room_id,
            access_token
        );
        let body = format!(r#"{{"body":"{}","msgtype":"m.text"}}"#, message);
        self.put(&create_event_path, &body)
    }

    /// Create a User and Room.
    pub fn initial_fixtures(&self, body: &str) -> (TestUser, String) {
        let user = self.create_user();
        let room_id = self.create_room_with_params(&user.token, body);
        (user, room_id)
    }

    /// Try to find a batch in a Response.
    pub fn get_next_batch(response: &Response) -> Batch {
        response
            .json()
            .find("next_batch")
            .unwrap()
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    /// Query sync with query parameter.
    pub fn sync(&self, access_token: &str, options: SyncOptions) -> Response {
        let mut path = match options.filter {
            Some(ref filter) => format!("/_matrix/client/r0/sync?filter={}&access_token={}", to_string(filter).unwrap(), access_token),
            None => format!("/_matrix/client/r0/sync?&access_token={}", access_token),
        };
        path = if options.full_state { format!("{}&full_state=true", path) } else { path };
        path = match options.set_presence {
            Some(PresenceState::Offline) => format!("{}&set_presence=offline", path),
            Some(PresenceState::Online) => format!("{}&set_presence=online", path),
            Some(PresenceState::Unavailable) => format!("{}&set_presence=unavailable", path),
            None => path,
        };
        path = match options.since {
            Some(batch) => format!("{}&since={}", path, batch.to_string()),
            None => path,
        };
        path = format!("{}&timeout={}", path, options.timeout);

        let response = self.get(&path);
        assert_eq!(response.status, Status::Ok);
        response
    }

    /// Test existent of keys in json.
    pub fn assert_json_keys(json: &Value, keys: Vec<&str>) {
        for key in keys.into_iter() {
            assert!(json.find(key).is_some());
        }
    }

    /// Update presence of a user.
    pub fn update_presence(&self, access_token: &str, user_id: &str, body: &str) -> Response {
        let presence_status_path = format!(
            "/_matrix/client/r0/presence/{}/status?access_token={}",
            user_id,
            access_token
        );
        let response = self.put(&presence_status_path , body);
        assert_eq!(response.status, Status::Ok);
        response
    }
}

impl Response {
    /// Creates a `Response` from an `iron::response::Response`.
    pub fn from_iron_response(response: iron::response::Response) -> Response {
        let headers = response.headers.clone();
        let status = response.status.expect("Response had no status").clone();
        let body = response::extract_body_to_string(response);

        let json = match from_str(&body) {
            Ok(json) => Some(json),
            _ => None,
        };

        Response {
            body: body,
            headers: headers,
            json: json,
            status: status,
        }
    }

    /// Returns the JSON in the response as a `serde_json::Value`. Panics if response body is not
    /// JSON.
    pub fn json(&self) -> &Value {
        self.json.as_ref().expect("Response did not contain JSON")
    }
}
