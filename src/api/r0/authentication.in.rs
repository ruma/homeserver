use iron::{Chain, Handler, IronResult, Plugin, Request, Response, status};
use serde_json::from_value;

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
        let registration_request: RegistrationRequest = itry!(
            from_value(request.extensions.get::<JsonRequest>().unwrap().clone()),
            APIError::bad_json()
        );

        Ok(Response::with((status::Ok, SerializableResponse("Registered!".to_owned()))))
    }
}

#[derive(Debug, Deserialize)]
struct RegistrationRequest {
    bind_email: Option<bool>,
    password: String,
    username: Option<String>,
}
