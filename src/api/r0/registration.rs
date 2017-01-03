//! Endpoints for user account registration.

use std::convert::TryFrom;

use bodyparser;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use ruma_identifiers::UserId;
use serde::de::{Deserialize, Deserializer, Visitor, Error as SerdeError};

use config::Config;
use crypto::hash_password;
use db::DB;
use error::ApiError;
use middleware::{JsonRequest, MiddlewareChain};
use models::user::{NewUser, User};
use modifier::SerializableResponse;

/// The `/register` endpoint.
pub struct Register;

#[derive(Clone, Debug, Deserialize)]
struct RegistrationRequest {
    pub bind_email: Option<bool>,
    pub kind: Option<RegistrationKind>,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Copy, Clone, Debug)]
enum RegistrationKind {
    Guest,
    User,
}

#[derive(Debug, Serialize)]
struct RegistrationResponse {
    pub access_token: String,
    pub home_server: String,
    pub user_id: String,
}

middleware_chain!(Register, [JsonRequest]);

impl Deserialize for RegistrationKind {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        struct RegistrationKindVisitor;

        impl Visitor for RegistrationKindVisitor {
            type Value = RegistrationKind;

            fn visit_str<E>(&mut self, value: &str) -> Result<RegistrationKind, E>
            where E: SerdeError {
                match value {
                    "guest" => Ok(RegistrationKind::Guest),
                    "user" => Ok(RegistrationKind::User),
                    _ => Err(SerdeError::custom(
                        r#"Parameter "kind" must be "guest" or "user""#
                    )),
                }
            }
        }

        deserializer.deserialize(RegistrationKindVisitor)
    }
}

impl Handler for Register {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let registration_request = match request.get::<bodyparser::Struct<RegistrationRequest>>() {
            Ok(Some(registration_request)) => registration_request,
            Ok(None) | Err(_) => {
                return Err(IronError::from(ApiError::bad_json(None)));
            }
        };

        if let Some(kind) = registration_request.kind {
            if let RegistrationKind::Guest = kind {
                return Err(IronError::from(ApiError::guest_forbidden(None)));
            }
        }

        let config = Config::from_request(request)?;

        let new_user = NewUser {
            id: match registration_request.username {
                Some(username) => {
                    UserId::try_from(&format!("@{}:{}", username, &config.domain))
                        .map_err(ApiError::from)?
                }
                None => UserId::new(&config.domain).map_err(ApiError::from)?,
            },
            password_hash: hash_password(&registration_request.password)?,
        };

        let connection = DB::from_request(request)?;

        if User::find_registered_user(&connection, &new_user.id)?.is_some() {
            let error = ApiError::unauthorized("This user_id already exists".to_string());

            return Err(IronError::from(error));
        }

        let (user, access_token) = User::create(
            &connection,
            &new_user,
            &config.macaroon_secret_key,
        )?;

        let response = RegistrationResponse {
            access_token: access_token.value,
            home_server: config.domain.clone(),
            user_id: user.id.to_string(),
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn minimum_input_parameters() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"password": "secret"}"#,
        );

        assert!(response.json().find("access_token").is_some());
        assert_eq!(response.json().find("home_server").unwrap().as_str().unwrap(), "ruma.test");
        assert!(response.json().find("user_id").is_some());
    }

    #[test]
    fn all_input_parameters() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username":"carl", "password": "secret"}"#
        );

        assert!(response.json().find("access_token").is_some());
        assert_eq!(response.json().find("home_server").unwrap().as_str().unwrap(), "ruma.test");
        assert_eq!(response.json().find("user_id").unwrap().as_str().unwrap(), "@carl:ruma.test");
    }

    #[test]
    fn guest_access_not_supported() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "guest", "username":"carl", "password": "secret"}"#
        );

        assert_eq!(
            response.json().find("errcode").unwrap().as_str().unwrap(),
            "M_GUEST_ACCESS_FORBIDDEN"
        );
    }

    #[test]
    fn user_already_registered() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username": "alice", "password": "secret"}"#
        );

        assert_eq!(response.status, Status::Ok);

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username": "alice", "password": "secret"}"#
        );

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().find("error").unwrap().as_str().unwrap(),
            "This user_id already exists"
        );
    }
}
