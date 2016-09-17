use bodyparser;
use diesel::SaveChangesDsl;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;

use crypto::hash_password;
use db::DB;
use error::ApiError;
use middleware::AccessTokenAuth;
use user::User;
use access_token::AccessToken;

/// The /account/password endpoint.
#[derive(Debug)]
pub struct AccountPassword;

#[derive(Clone, Debug, Deserialize)]
struct AccountPasswordRequest {
    pub new_password: String,
}

/// The /account/deactivate endpoint.
#[derive(Debug)]
pub struct DeactivateAccount;

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
                let error = ApiError::not_json(None);

                return Err(IronError::new(error.clone(), error));
            }
        };

        let mut user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        user.password_hash = hash_password(&account_password_request.new_password)?;

        let connection = DB::from_request(request)?;

        if let Err(_) = user.save_changes::<User>(&*connection) {
            let error = ApiError::unauthorized(None);

            return Err(IronError::new(error.clone(), error));
        }

        Ok(Response::with(Status::Ok))
    }
}

impl DeactivateAccount {
    /// Create a `DeactivateAccount` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(DeactivateAccount);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for DeactivateAccount {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let connection = DB::from_request(request)?;

        {
            let token = request.extensions.get_mut::<AccessToken>()
                .expect("AccessTokenAuth should ensure an access token");

            if let Err(error) = token.revoke(&connection) {
                return Err(IronError::new(error.clone(), error));
            };
        }

        let user = request.extensions.get_mut::<User>()
            .expect("AccessTokenAuth should ensure a user");

        if let Err(error) = user.deactivate(&connection) {
            return Err(IronError::new(error.clone(), error));
        };

        // TODO: Delete 3pid for the user

        Ok(Response::with(Status::Ok))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn change_password() {
        let test = Test::new();
        let access_token = test.create_access_token();

        assert!(
            test.post(
                &format!("/_matrix/client/r0/account/password?access_token={}", access_token),
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

    #[test]
    fn deactivate_account() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let login = r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "secret"}}"#;
        let deactivate = format!("/_matrix/client/r0/account/deactivate?access_token={}", access_token);

        assert!(test.post("/_matrix/client/r0/login", login).status.is_success());

        assert!(test.post(&deactivate, r#"{}"#).status.is_success());

        assert_eq!(test.post("/_matrix/client/r0/login", login).status, Status::Forbidden);
        assert_eq!(test.post(&deactivate, r#"{}"#).status, Status::Forbidden);
    }
}
