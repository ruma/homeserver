use iron::{Chain, Handler, IronResult, Request, Response, status};

use ruma_identifiers::UserId;

use authentication::{AuthType, Flow, InteractiveAuth};
use config::Config;
use db::DB;
use middleware::{JsonRequest, MiddlewareChain, UIAuth};
use models::access_token::AccessToken;
use models::user::User;
use modifier::SerializableResponse;

/// The `/login` endpoint.
pub struct Login;

#[derive(Debug, Serialize)]
struct LoginResponse {
    /// An access token for the account. This access token can then be used to authorize other requests.
    pub access_token: String,
    /// The hostname of the homeserver on which the account has been registered.
    pub home_server: String,
    /// The fully-qualified Matrix ID that has been registered.
    pub user_id: UserId,
}

middleware_chain!(Login, [JsonRequest, UIAuth::new(InteractiveAuth::new(vec![Flow::new(vec![AuthType::Password])]))]);

impl Handler for Login {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>().expect("UIAuth should ensure a user").clone();
        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;
        let access_token = AccessToken::create(&connection, &user.id, &config.macaroon_secret_key)?;

        let response = LoginResponse {
            access_token: access_token.value,
            home_server: config.domain.clone(),
            user_id: user.id,
        };

        Ok(Response::with((status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn valid_credentials() {
        let test = Test::new();

        assert!(test.register_user(
            r#"{"username": "carl", "password": "secret"}"#
        ).status.is_success());

        let response = test.post(
            "/_matrix/client/r0/login",
            r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "secret"}}"#,
        );

        assert!(response.json().find("access_token").is_some());
        assert_eq!(response.json().find("home_server").unwrap().as_str().unwrap(), "ruma.test");
        assert_eq!(response.json().find("user_id").unwrap().as_str().unwrap(), "@carl:ruma.test");
    }

    #[test]
    fn invalid_credentials() {
        let test = Test::new();

        let response = test.register_user(r#"{"username": "carl", "password": "secret"}"#);
        assert_eq!(response.status, Status::Ok);

        let response = test.post(
            "/_matrix/client/r0/login",
            r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "another_secret"}}"#,
        );

        assert_eq!(response.status, Status::Forbidden);
    }

    #[test]
    fn login_without_register() {
        let test = Test::new();

        let response = test.post(
            "/_matrix/client/r0/login",
            r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "secret"}}"#,
        );

        assert_eq!(response.status, Status::Forbidden);
    }
}
