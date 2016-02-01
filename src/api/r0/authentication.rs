use bodyparser;
use diesel::query_builder::insert;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response, status};

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
        let registration_request = match request.get::<bodyparser::Struct<RegistrationRequest>>() {
            Ok(Some(registration_request)) => registration_request,
            Ok(None) | Err(_) => {
                let error = APIError::not_json();

                return Err(IronError::new(error.clone(), error));
            }
        };

        Ok(Response::with((status::Ok, SerializableResponse("Registered!".to_owned()))))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[insertable_into(users)]
struct RegistrationRequest {
    bind_email: Option<bool>,
    password: String,
    username: Option<String>,
}
