use iron::{Chain, Handler, IronResult, Request, Response};
use iron::status::Status;

use access_token::AccessToken;
use db::DB;
use middleware::AccessTokenAuth;

/// The /logout endpoint.
pub struct Logout;

impl Logout {
    /// Create a `Logout` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Logout);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for Logout {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let connection = DB::from_request(request)?;

        let access_token = request.extensions.get_mut::<AccessToken>()
            .expect("AccessTokenAuth should ensure an access token");

        access_token.revoke(&connection)?;

        Ok(Response::with(Status::Ok))
    }
}

#[cfg(test)]
mod tests {
    use iron::status::Status;

    use test::Test;

    #[test]
    fn logout_revokes_access_token() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/register",
            r#"{"username": "carl", "password": "secret"}"#,
        );

        let access_token = response.json().find("access_token").unwrap().as_string().unwrap();
        let login_path = format!("/_matrix/client/r0/logout?token={}", access_token);

        assert!(test.post(&login_path, "{}").status.is_success());
        assert_eq!(test.post(&login_path, "{}").status, Status::Forbidden);
    }
}
