//! Endpoints for user account registration.

use bodyparser;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use serde::de::{Deserialize, Deserializer, Visitor, Error as SerdeError};

use config::get_config;
use crypto::hash_password;
use db::get_connection;
use error::APIError;
use middleware::JsonRequest;
use modifier::SerializableResponse;
use user::{NewUser, generate_user_id, insert_user};

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

/// The /register endpoint.
pub struct Register;

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

impl Register {
    /// Create a `Register` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Register);

        chain.link_before(JsonRequest);

        chain
    }
}

impl Handler for Register {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let registration_request = match request.get::<bodyparser::Struct<RegistrationRequest>>() {
            Ok(Some(registration_request)) => registration_request,
            Ok(None) | Err(_) => {
                let error = APIError::not_json();

                return Err(IronError::new(error.clone(), error));
            }
        };

        if let Some(kind) = registration_request.kind {
            match kind {
                RegistrationKind::Guest => {
                    let error = APIError::guest_forbidden();

                    return Err(IronError::new(error.clone(), error));
                }
                _ => {},
            }
        }

        let new_user = NewUser {
            id: registration_request.username.unwrap_or(generate_user_id()),
            password_hash: hash_password(&registration_request.password)?,
        };

        let connection = get_connection(request)?;
        let config = get_config(request)?;

        let (user, access_token) = insert_user(
            &connection,
            &new_user,
            &config.macaroon_secret_key,
        )?;

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
    use test::Test;

    #[test]
    fn minimum_input_parameters() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/register",
            r#"{"password": "secret"}"#,
        );

        assert!(response.json.find("access_token").is_some());
        assert_eq!(response.json.find("home_server").unwrap().as_string().unwrap(), "ruma.test");
        assert!(response.json.find("user_id").is_some());
    }

    #[test]
    fn all_input_parameters() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/register",
            r#"{"bind_email": true, "kind": "user", "username":"carl", "password": "secret"}"#,
        );

        assert!(response.json.find("access_token").is_some());
        assert_eq!(response.json.find("home_server").unwrap().as_string().unwrap(), "ruma.test");
        assert_eq!(response.json.find("user_id").unwrap().as_string().unwrap(), "carl");
    }

    #[test]
    fn guest_access_not_supported() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/register",
            r#"{"bind_email": true, "kind": "guest", "username":"carl", "password": "secret"}"#,
        );

        assert_eq!(
            response.json.find("errcode").unwrap().as_string().unwrap(),
            "M_GUEST_ACCESS_FORBIDDEN"
        );
    }
}
