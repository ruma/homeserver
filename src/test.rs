use diesel::Connection;
use diesel::pg::PgConnection;
use iron::headers::{ContentType, Headers};
use iron::status::Status;
use iron_test::{request, response};
use mount::Mount;
use r2d2::{Config as R2D2Config, CustomizeConnection};
use r2d2_diesel::Error as R2D2DieselError;
use serde_json::{Value, from_str};

use config::FinalConfig;
use server::Server;

const POSTGRES_URL: &'static str = "postgres://postgres:test@127.0.0.1:5432/postgres";

pub struct Test {
    mount: Mount,
}

pub struct Response {
    pub body: String,
    pub headers: Headers,
    pub json: Value,
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
        let config = FinalConfig {
            bind_address: "127.0.0.1".to_string(),
            bind_port: "0".to_string(),
            domain: "ruma.test".to_string(),
            macaroon_secret_key: "YymznQHmKdN9B4f7iBalJB1tWEDy9LdaFSQJEtB3R5w=".into(),
            postgres_url: POSTGRES_URL.to_string(),
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

    pub fn post(&self, path: &str, body: &str) -> Response {
        let mut headers = Headers::new();

        headers.set(ContentType::json());

        let response = match request::post(
            &format!("http://ruma.test{}", path)[..],
            headers,
            body,
            &self.mount,
        ) {
            Ok(response) => response,
            Err(error)  => error.response,
        };

        let headers = response.headers.clone();
        let status = response.status.expect("Response had no status").clone();
        let body = response::extract_body_to_string(response);

        let json = match from_str(&body) {
            Ok(json) => json,
            Err(error) => panic!("Failed to parse response as JSON: {}", error),
        };

        Response {
            body: body,
            headers: headers,
            json: json,
            status: status,
        }
    }
}
