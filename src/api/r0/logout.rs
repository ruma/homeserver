//! Endpoints for logging out users.

use iron::status::Status;
use iron::{Chain, Handler, IronResult, Request, Response};

use crate::db::DB;
use crate::middleware::{AccessTokenAuth, MiddlewareChain};
use crate::models::access_token::AccessToken;
use crate::modifier::EmptyResponse;

/// The `/logout` endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Logout;

middleware_chain!(Logout, [AccessTokenAuth]);

impl Handler for Logout {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let connection = DB::from_request(request)?;

        let access_token = request
            .extensions
            .get_mut::<AccessToken>()
            .expect("AccessTokenAuth should ensure an access token");

        access_token.revoke(&connection)?;

        Ok(Response::with(EmptyResponse(Status::Ok)))
    }
}

#[cfg(test)]
mod tests {
    use iron::status::Status;

    use crate::test::Test;

    #[test]
    fn logout_revokes_access_token() {
        let test = Test::new();
        let user = test.create_user();

        let login_path = format!("/_matrix/client/r0/logout?access_token={}", user.token);

        assert!(test.post(&login_path, "{}").status.is_success());
        assert_eq!(test.post(&login_path, "{}").status, Status::Forbidden);
    }
}
