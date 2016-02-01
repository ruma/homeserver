use bodyparser;
use diesel::query_builder::insert;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};
use rand::{Rng, thread_rng};

use error::APIError;
use middleware::JsonRequest;
use modifier::SerializableResponse;

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
        let mut registration_request = match request.get::<bodyparser::Struct<RegistrationRequest>>() {
            Ok(Some(registration_request)) => registration_request,
            Ok(None) | Err(_) => {
                let error = APIError::not_json();

                return Err(IronError::new(error.clone(), error));
            }
        };

        if registration_request.username.is_none() {
            registration_request.username = Some(thread_rng().gen_ascii_chars().take(12).collect());
        }

        Ok(Response::with((status::Ok, SerializableResponse("Registered!".to_owned()))))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[insertable_into(users)]
struct RegistrationRequest {
    pub bind_email: Option<bool>,
    pub password: String,
    pub username: Option<String>,
}
