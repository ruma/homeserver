use iron::{IronResult, Request, Response, status};

pub fn register(_request: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Registered!")))
}
