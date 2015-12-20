use iron::{Chain, Handler, IronResult, Request, Response, status};

use middleware::{InteractiveAuthentication, JsonRequest};

pub struct Register;

impl Register {
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Register);

        chain.link_before(JsonRequest);
        chain.link_before(InteractiveAuthentication);

        chain
    }
}

impl Handler for Register {
    fn handle(&self, _request: &mut Request) -> IronResult<Response> {
        Ok(Response::with((status::Ok, "Registered!")))
    }
}
