use std::io::Read;
use std::fmt::Display;
use std::process::{Command, Stdio};
use std::sync::{Once, ONCE_INIT};
use std::sync::mpsc::channel;
use std::thread::{Builder, JoinHandle};

use env_logger::LogBuilder;
use hyper::client::{Body, Client, IntoUrl};
use hyper::header::{ContentType, Headers};
use hyper::status::StatusCode;

use ruma::config::FinalConfig;
use ruma::server::Server;

static START: Once = ONCE_INIT;

pub struct Test {
    client: Client,
    postgres_container_name: String,
    // Must keep a reference to this so the thread stays alive until the struct is dropped.
    #[allow(dead_code)]
    server_thread: JoinHandle<()>,
    server_thread_port: String,
}

pub struct Response {
    pub body: String,
    pub headers: Headers,
    pub status: StatusCode,
}

impl Test {
    pub fn new() -> Self {
        START.call_once(|| {
            let mut builder = LogBuilder::new();

            builder.parse("ruma=trace,diesel=trace");

            builder.init().expect("Failed to initialize logger");
        });

        let docker_postgres = match Command::new("docker").args(&[
            "run",
            "-d",
            "-e",
            "POSTGRES_PASSWORD=test",
            "-P",
            "postgres",
        ]).output() {
            Ok(output) => output,
            Err(error) => panic!("`docker run postgres` failed: {}", error),
        };

        let postgres_container_name = String::from_utf8(docker_postgres.stdout).expect(
            "`docker run` output was not valid UTF-8"
        ).trim_right().to_string();

        let postgres_container_host_ip = String::from_utf8(
            match Command::new("docker").args(&[
                "inspect",
                "-f",
                "{{(index (index .NetworkSettings.Ports \"5432/tcp\") 0).HostIp}}",
                &postgres_container_name,
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
                &postgres_container_name,
            ]).output() {
                Ok(output) => output.stdout,
                Err(error) => panic!("`docker inspect postgres` for port failed: {}", error),
            }
        ).expect("`docker inspect` output was not valid UTF-8").trim_right().to_string();

        let config = FinalConfig {
            bind_address: "127.0.0.1".to_string(),
            bind_port: "0".to_string(),
            domain: "ruma.test".to_string(),
            macaroon_secret_key: "YymznQHmKdN9B4f7iBalJB1tWEDy9LdaFSQJEtB3R5w=".into(),
            postgres_url: format!(
                "postgres://postgres:test@{}:{}/postgres",
                &postgres_container_host_ip,
                &postgres_container_host_port,
            ),
        };

        let (tx, rx) = channel();

        let server_thread = match Builder::new().name("iron".to_string()).spawn(move || {
            let server = match Server::new(&config) {
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
            postgres_container_name: postgres_container_name,
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

                Response {
                    body: body,
                    headers: response.headers.clone(),
                    status: response.status.clone(),
                }
            }
            Err(error)  => panic!("{}", error),
        }
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        let exit_status = Command::new("docker").args(&[
            "rm",
            "-f",
            "-v",
            &self.postgres_container_name,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status().ok().expect(&format!(
            "Failed to remove PostgreSQL container {}",
            &self.postgres_container_name,
        ));

        assert!(exit_status.success());
    }
}
