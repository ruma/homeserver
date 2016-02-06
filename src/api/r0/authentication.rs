use std::error::Error;

use bodyparser;
use diesel::{ExecuteDsl, insert};
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use persistent::Write;
use rand::{Rng, thread_rng};

use db::DB;
use error::APIError;
use middleware::JsonRequest;
use modifier::SerializableResponse;
use users;

#[derive(Clone, Debug, Deserialize)]
struct RegistrationRequest {
    pub bind_email: Option<bool>,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Debug)]
#[insertable_into(users)]
struct NewUser {
    id: String,
    password_hash: String,
}

pub struct Register;

impl Register {
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
            password_hash: registration_request.password,
        };

        let pool_mutex = try!(request.get::<Write<DB>>().map_err(APIError::from));
        let pool = try!(pool_mutex.lock().map_err(|error| {
            APIError::unknown_from_string(format!("{}", error))
        }));
        let connection = try!(pool.get().map_err(APIError::from));

        match insert(&new_user).into(users::table).execute(&*connection) {
            Ok(_) => {
                Ok(Response::with((status::Ok, SerializableResponse("Registered!".to_owned()))))
            }
            Err(diesel_error) => {
                let error = APIError::unknown(&diesel_error);

                Err(IronError::new(error.clone(), error))
            }
        }
    }
}
