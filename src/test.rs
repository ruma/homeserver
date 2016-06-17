use std::sync::{ONCE_INIT, Once};

use env_logger;
use diesel::Connection;
use diesel::migrations::{run_pending_migrations, setup_database};
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
use server::Server;

static START: Once = ONCE_INIT;
const DATABASE_URL: &'static str = "postgres://postgres:test@postgres:5432/ruma_test";
const POSTGRES_URL: &'static str = "postgres://postgres:test@postgres:5432";

pub struct Test {
    mount: Mount,
}

#[derive(Debug)]
pub struct Response {
    pub body: String,
    pub headers: Headers,
    json: Option<Value>,
    pub status: Status,
}

#[derive(Debug)]
pub struct TestTransactionConnectionCustomizer;

impl CustomizeConnection<PgConnection, R2D2DieselError> for TestTransactionConnectionCustomizer {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), R2D2DieselError> {
        conn.begin_test_transaction().map_err(|error| R2D2DieselError::QueryError(error))
    }
}

impl Test {
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

    pub fn get(&self, path: &str) -> Response {
        self.request(Method::Get, path, "")
    }

    pub fn post(&self, path: &str, body: &str) -> Response {
        self.request(Method::Post, path, body)
    }

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

    pub fn register_user(&self, body: &str) -> Response {
        self.post("/_matrix/client/r0/register", body)
    }

    pub fn create_access_token(&self) -> String {
        self.register_user(r#"{"username": "carl", "password": "secret"}"#)
            .json()
            .find("access_token")
            .unwrap()
            .as_string()
            .unwrap()
            .to_string()
    }
}

impl Response {
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

    pub fn json(&self) -> &Value {
        self.json.as_ref().expect("Response did not contain JSON")
    }
}
