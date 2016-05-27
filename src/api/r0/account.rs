use bodyparser;
use diesel::SaveChangesDsl;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;

use crypto::hash_password;
use db::DB;
use error::APIError;
use middleware::AccessTokenAuth;
use user::User;

/// The /account/password endpoint.
#[derive(Debug)]
pub struct AccountPassword;

#[derive(Clone, Debug, Deserialize)]
struct AccountPasswordRequest {
    pub new_password: String,
}

impl AccountPassword {
    /// Create an `AccountPassword` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(AccountPassword);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for AccountPassword {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let account_password_request = match request
            .get::<bodyparser::Struct<AccountPasswordRequest>>()
        {
            Ok(Some(account_password_request)) => account_password_request,
            Ok(None) | Err(_) => {
                let error = APIError::not_json();

                return Err(IronError::new(error.clone(), error));
            }
        };

        let mut user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        user.password_hash = hash_password(&account_password_request.new_password)?;

        let connection = DB::from_request(request)?;

        if let Err(_) = user.save_changes::<User>(&connection) {
            let error = APIError::unauthorized();

            return Err(IronError::new(error.clone(), error));
        }

        Ok(Response::with(Status::Ok))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;

    #[test]
    fn change_password() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/register",
            r#"{"username": "carl", "password": "secret"}"#,
        );

        let access_token = response.json().find("access_token").unwrap().as_string().unwrap();

        assert!(
            test.post(
                &format!("/_matrix/client/r0/account/password?token={}", access_token),
                r#"{"new_password": "hidden"}"#
            ).status.is_success()
        );

        assert!(
            test.post(
                "/_matrix/client/r0/login",
                r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "hidden"}}"#,
            ).status.is_success()
        )
    }
}
