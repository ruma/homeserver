use std::io::Read;
use std::env::var;
use std::fmt::Display;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::thread::{Builder, JoinHandle};

use diesel::Connection;
use diesel::migrations::{run_pending_migrations, setup_database};
use diesel::pg::PgConnection;
use env_logger::LogBuilder;
use hyper::client::{Body, Client, IntoUrl};
use hyper::header::{ContentType, Headers};
use hyper::status::StatusCode;
use r2d2::{Config as R2D2Config, CustomizeConnection};
use r2d2_diesel::Error as R2D2DieselError;
use serde_json::{Value, from_str};

use config::FinalConfig;
use server::Server;
use db::create_connection_pool;

pub mod registration;

lazy_static! {
    pub static ref POSTGRES_CONTAINER: PostgresContainer = {
        // Set up logging
        let mut builder = LogBuilder::new();

        match var("RUST_LOG") {
            Ok(directives) => { builder.parse(&directives); }
            Err(_) => { builder.parse("ruma=error"); }
        }

        builder.init().expect("Failed to initialize logger");

        // Set up Postgres Docker container
        match Command::new("docker").args(&[
            "run",
            "-d",
            "-e",
            "POSTGRES_PASSWORD=test",
            "--name",
            "ruma_test_postgres",
            "-P",
            "postgres",
        ]).output() {
            Ok(output) => output,
            Err(error) => panic!("`docker run postgres` failed: {}", error),
        };

        let postgres_container_host_ip = String::from_utf8(
            match Command::new("docker").args(&[
                "inspect",
                "-f",
                "{{(index (index .NetworkSettings.Ports \"5432/tcp\") 0).HostIp}}",
                "ruma_test_postgres",
            ]).output() {
                Ok(output) => output.stdout,
                Err(error) => panic!("`docker inspect postgres` for IP failed: {}", error),
            }
        ).expect("`docker inspect` output was not valid UTF-8").trim_right().to_string();

        let postgres_container_host_port = String::from_utf8(
            match Command::new("docker").args(&[
                "inspect",
                "-f",
                "{{(index (index .NetworkSettings.Ports \"5432/tcp\") 0).HostPort}}",
                "ruma_test_postgres",
            ]).output() {
                Ok(output) => output.stdout,
                Err(error) => panic!("`docker inspect postgres` for port failed: {}", error),
            }
        ).expect("`docker inspect` output was not valid UTF-8").trim_right().to_string();

        let postgres_container = PostgresContainer {
            url: format!(
                "postgres://postgres:test@{}:{}/postgres",
                &postgres_container_host_ip,
                &postgres_container_host_port,
            ),
        };

        let connection_pool = create_connection_pool(
            R2D2Config::default(),
            &postgres_container.url
        ).expect("Failed to create PostgreSQL connection pool.");

        let connection = connection_pool.get().expect(
            "Failed to get PostgreSQL connection pool to set up database and run migrations."
        );

        setup_database(&*connection).expect("Failed to set up database.");

        run_pending_migrations(&*connection).expect("Failed to run migrations.");

        postgres_container
    };
}

pub struct PostgresContainer {
    url: String,
}

// TODO: Find a way to actually execute this. Apparently the lazy static doesn't get dropped. :(
impl Drop for PostgresContainer {
    fn drop(&mut self) {
        println!("DROP IS RUNNING OMG");
        let exit_status = Command::new("docker").args(&["rm", "-f", "-v", "ruma_test_postgres"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status().ok().expect("Failed to remove PostgreSQL container ruma_test_postgres");

        assert!(exit_status.success());
    }
}

pub struct Test {
    client: Client,
    // Must keep a reference to this so the thread stays alive until the struct is dropped.
    #[allow(dead_code)]
    server_thread: JoinHandle<()>,
    server_thread_port: String,
}

pub struct Response {
    pub body: String,
    pub headers: Headers,
    pub json: Value,
    pub status: StatusCode,
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
        let (tx, rx) = channel();

        let server_thread = match Builder::new().name("iron".to_string()).spawn(move || {
            let config = FinalConfig {
                bind_address: "127.0.0.1".to_string(),
                bind_port: "0".to_string(),
                domain: "ruma.test".to_string(),
                macaroon_secret_key: "YymznQHmKdN9B4f7iBalJB1tWEDy9LdaFSQJEtB3R5w=".into(),
                postgres_url: POSTGRES_CONTAINER.url.clone(),
            };

            let r2d2_config = R2D2Config::builder()
                .pool_size(1)
                .connection_customizer(Box::new(TestTransactionConnectionCustomizer))
                .build();

            let server = match Server::with_options(&config, r2d2_config, false) {
                Ok(server) => server,
                Err(error) => panic!("Failed to create Iron server: {}", error),
            };

            let listening = match server.run() {
                Ok(listening) => listening,
                Err(error) => panic!("Failed to run Iron server: {}", error),
            };

            if let Err(error) = tx.send(listening.socket.port().to_string()) {
                panic!("Failed to send Iron server port to main thread: {}", error);
            }
        }) {
            Ok(server_thread) => server_thread,
            Err(error) => panic!("Failed to create thread for Iron server: {}", error),
        };

        let server_thread_port = match rx.recv() {
            Ok(server_thread_port) => server_thread_port,
            Err(error) => panic!("Failed to receive Iron server port: {}", error),
        };

        Test {
            client: Client::new(),
            server_thread: server_thread,
            server_thread_port: server_thread_port,
        }
    }

    pub fn post<'a, U, B>(&'a self, url: U, body: B) -> Response
    where U: Display + IntoUrl, B: Into<Body<'a>> {
        let uri = format!("http://127.0.0.1:{}{}", self.server_thread_port, url);

        match self.client.post(&uri).header(ContentType::json()).body(body).send() {
            Ok(mut response) => {
                let mut body = String::new();

                if let Err(error) = response.read_to_string(&mut body) {
                    panic!("Failed to read HTTP response body: {}", error);
                }

                let json = match from_str(&body) {
                    Ok(json) => json,
                    Err(error) => panic!("Failed to parse response as JSON: {}", error),
                };

                Response {
                    body: body,
                    headers: response.headers.clone(),
                    json: json,
                    status: response.status.clone(),
                }
            }
            Err(error)  => panic!("{}", error),
        }
    }
}
