//! Endpoints for user authentication.

use std::error::Error;

use base64::u8en;
use argon2rs::argon2i_simple;
use bodyparser;
use diesel::{LoadDsl, insert};
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use persistent::Read;
use rand::{Rng, thread_rng};

use config::FinalConfig;
use db::get_connection;
use error::APIError;
use middleware::JsonRequest;
use modifier::SerializableResponse;
use schema::users;
use user::{NewUser, User};


#[derive(Clone, Debug, Deserialize)]
struct RegistrationRequest {
    pub bind_email: Option<bool>,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegistrationResponse {
    pub access_token: String,
    pub home_server: String,
    pub user_id: String,
}

/// The /register endpoint.
pub struct Register;

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

        let new_user = NewUser {
            id: registration_request.username.unwrap_or(
                thread_rng().gen_ascii_chars().take(12).collect()
            ),
            password_hash: try!(
                String::from_utf8(
                    try!(
                        u8en(
                            &argon2i_simple(&registration_request.password, "extremely insecure")
                        ).map_err(APIError::from)
                    )
                ).map_err(APIError::from)
            ),
        };

        let connection = try!(get_connection(request));

        let user: User = try!(
            insert(&new_user).into(users::table).get_result(&*connection).map_err(APIError::from)
        );

        let config = try!(request.get_ref::<Read<FinalConfig>>().map_err(APIError::from));

        Ok(
            Response::with((
                status::Ok,
                SerializableResponse(RegistrationResponse {
                    access_token: "fake access token".to_owned(),
                    home_server: config.domain.clone(),
                    user_id: user.id,
                })
            ))
        )
    }
}
