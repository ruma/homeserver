use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::thread::{Builder, JoinHandle};

use ruma::config::FinalConfig;
use ruma::server::Server;

pub struct RumaTest {
    postgres_container_name: String,
    server_thread: JoinHandle<()>,
    server_thread_port: String,
}

impl RumaTest {
    pub fn new() -> Self {
        let docker_postgres = Command::new("docker").args(&[
            "run",
            "-d",
            "-e",
            "POSTGRES_PASSWORD=test",
            "-P",
            "postgres",
        ]).output().ok().expect("`docker run postgres` failed");

        let postgres_container_name = String::from_utf8(docker_postgres.stdout).expect(
            "`docker run` output was not valid UTF-8"
        ).trim_right().to_string();

        let postgres_container_host_ip = String::from_utf8(
            Command::new("docker").args(&[
                "inspect",
                "-f",
                "{{(index (index .NetworkSettings.Ports \"5432/tcp\") 0).HostIp}}",
                &postgres_container_name,
            ]).output().ok().expect("`docker inspect postgres` for IP failed").stdout
        ).expect("`docker inspect` output was not valid UTF-8").trim_right().to_string();

        let postgres_container_host_port = String::from_utf8(
            Command::new("docker").args(&[
                "inspect",
                "-f",
                "{{(index (index .NetworkSettings.Ports \"5432/tcp\") 0).HostPort}}",
                &postgres_container_name,
            ]).output().ok().expect("`docker inspect postgres` for port failed").stdout
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

        let server_thread = Builder::new().name("iron".to_string()).spawn(move || {
            let server = Server::new(&config).ok().expect("Failed to create Iron server");
            let listening = server.run().ok().expect("Failed to run Iron server");

            tx.send(listening.socket.port().to_string()).expect(
                "Failed to send Iron server port to main thread"
            );
        }).expect("Failed to create thread for Iron server");

        let server_thread_port = rx.recv().expect("Failed to receive Iron server port");

        RumaTest {
            postgres_container_name: postgres_container_name,
            server_thread: server_thread,
            server_thread_port: server_thread_port,
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://ruma.test:{}", self.server_thread_port)
    }
}

impl Drop for RumaTest {
    fn drop(&mut self) {
        Command::new("docker").args(&[
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
    }
}
