use iron::{Chain, Handler, IronResult, Request, Response, status};

use middleware::JsonRequest;
use modifier::SerializableResponse;

/// The /login endpoint.
pub struct Login;

impl Login {
    /// Create a `Login` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Login);

        chain.link_before(JsonRequest);

        chain
    }
}

impl Handler for Login {
    fn handle(&self, _request: &mut Request) -> IronResult<Response> {
        Ok(Response::with((status::Ok, SerializableResponse("{}"))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;

    #[test]
    fn valid_credentials() {
        let test = Test::new();

        assert!(test.post(
            "/_matrix/client/r0/register",
            r#"{"username": "carl", "password": "secret"}"#,
        ).status.is_success());

        let response = test.post(
            "/_matrix/client/r0/login",
            r#"{"user": "carl", "password": "secret"}"#,
        );

        assert!(response.json.find("access_token").is_some());
    }
}
