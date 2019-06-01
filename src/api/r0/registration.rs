//! Endpoints for user account registration.

use std::convert::TryFrom;
use std::fmt::{Formatter, Result as FmtResult};

use bodyparser;
use iron::{status, Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use ruma_identifiers::UserId;
use serde::de::{Deserialize, Deserializer, Error as SerdeError, Visitor};

use crate::config::Config;
use crate::crypto::hash_password;
use crate::db::DB;
use crate::error::ApiError;
use crate::middleware::{JsonRequest, MiddlewareChain};
use crate::models::profile::Profile;
use crate::models::user::{NewUser, User};
use crate::modifier::SerializableResponse;

/// The `/register` endpoint.
pub struct Register;

#[derive(Clone, Debug, Deserialize)]
struct RegistrationRequest {
    /// If true, the server binds the email used for authentication to the Matrix ID with the ID Server.
    pub bind_email: Option<bool>,
    /// The kind of account to register. Defaults to user. One of: ["guest", "user"]
    pub kind: Option<RegistrationKind>,
    /// The desired password for the account.
    pub password: String,
    /// The local part of the desired Matrix ID. If omitted, the homeserver
    /// MUST generate a Matrix ID local part.
    pub username: Option<String>,
}

#[derive(Copy, Clone, Debug)]
enum RegistrationKind {
    Guest,
    User,
}

#[derive(Debug, Serialize)]
struct RegistrationResponse {
    /// An access token for the account. This access token can then be used to authorize other requests.
    pub access_token: String,
    /// The hostname of the homeserver on which the account has been registered.
    pub home_server: String,
    /// The fully-qualified Matrix ID that has been registered.
    pub user_id: UserId,
}

middleware_chain!(Register, [JsonRequest]);

impl<'de> Deserialize<'de> for RegistrationKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RegistrationKindVisitor;

        impl<'de> Visitor<'de> for RegistrationKindVisitor {
            type Value = RegistrationKind;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                write!(formatter, "a registration kind")
            }

            fn visit_str<E>(self, value: &str) -> Result<RegistrationKind, E>
            where
                E: SerdeError,
            {
                match value {
                    "guest" => Ok(RegistrationKind::Guest),
                    "user" => Ok(RegistrationKind::User),
                    _ => Err(SerdeError::custom(
                        r#"Parameter "kind" must be "guest" or "user""#,
                    )),
                }
            }
        }

        deserializer.deserialize_any(RegistrationKindVisitor)
    }
}

impl Handler for Register {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let registration_request = match request.get::<bodyparser::Struct<RegistrationRequest>>() {
            Ok(Some(registration_request)) => registration_request,
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
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
                    UserId::try_from(format!("@{}:{}", username, &config.domain).as_ref())
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

        let (user, access_token) =
            User::create(&connection, &new_user, &config.macaroon_secret_key)?;

        let new_profile = Profile {
            id: user.id.clone(),
            avatar_url: None,
            displayname: None,
        };

        Profile::create(&connection, &new_profile)?;

        let response = RegistrationResponse {
            access_token: access_token.value,
            home_server: config.domain.clone(),
            user_id: user.id,
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::Test;
    use iron::status::Status;

    #[test]
    fn minimum_input_parameters() {
        let test = Test::new();

        let response = test.register_user(r#"{"password": "secret"}"#);

        assert!(response.json().get("access_token").is_some());
        assert_eq!(
            response
                .json()
                .get("home_server")
                .unwrap()
                .as_str()
                .unwrap(),
            "ruma.test"
        );
        assert!(response.json().get("user_id").is_some());
    }

    #[test]
    fn all_input_parameters() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username":"carl", "password": "secret"}"#,
        );

        assert!(response.json().get("access_token").is_some());
        assert_eq!(
            response
                .json()
                .get("home_server")
                .unwrap()
                .as_str()
                .unwrap(),
            "ruma.test"
        );
        assert_eq!(
            response.json().get("user_id").unwrap().as_str().unwrap(),
            "@carl:ruma.test"
        );
    }

    #[test]
    fn guest_access_not_supported() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "guest", "username":"carl", "password": "secret"}"#,
        );

        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "M_GUEST_ACCESS_FORBIDDEN"
        );
    }

    #[test]
    fn user_already_registered() {
        let test = Test::new();

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username": "alice", "password": "secret"}"#,
        );

        assert_eq!(response.status, Status::Ok);

        let response = test.register_user(
            r#"{"bind_email": true, "kind": "user", "username": "alice", "password": "secret"}"#,
        );

        assert_eq!(response.status, Status::Forbidden);
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "This user_id already exists"
        );
    }
}
