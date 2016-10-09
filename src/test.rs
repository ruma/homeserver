use std::sync::{ONCE_INIT, Once};

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
use serde_json::{Value, from_str};

use config::Config;
use embedded_migrations::run as run_pending_migrations;
use server::Server;

static START: Once = ONCE_INIT;
const DATABASE_URL: &'static str = "postgres://postgres:test@postgres:5432/ruma_test";
const POSTGRES_URL: &'static str = "postgres://postgres:test@postgres:5432";

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
            Err(error)  => error.response,
        };

        Response::from_iron_response(response)
    }

    /// Registers a new user account and returns the response of the API call.
    pub fn register_user(&self, body: &str) -> Response {
        self.post("/_matrix/client/r0/register", body)
    }

    /// Registers a new user account with a fixed name and returns the user's access token.
    pub fn create_access_token(&self) -> String {
        self.create_access_token_with_username("carl")
    }

    /// Registers a new user account with the given username and returns the user's access token.
    pub fn create_access_token_with_username(&self, username: &str) -> String {
        self.register_user(&format!(r#"{{"username": "{}", "password": "secret"}}"#, username))
            .json()
            .find("access_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Creates a room and returns the room ID as a string.
    pub fn create_room(&self, access_token: &str) -> String {
        self.post(&format!("/_matrix/client/r0/createRoom?access_token={}", access_token), "{}")
            .json()
            .find("room_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Creates a room and returns the room ID as a string.
    pub fn create_public_room(&self, access_token: &str) -> String {
        self.post(&format!("/_matrix/client/r0/createRoom?access_token={}", access_token), r#"{"visibility": "public"}"#)
            .json()
            .find("room_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Creates a room and returns the room ID as a string.
    pub fn create_private_room(&self, access_token: &str) -> String {
        self.post(&format!("/_matrix/client/r0/createRoom?access_token={}", access_token), r#"{"visibility": "private"}"#)
            .json()
            .find("room_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
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
