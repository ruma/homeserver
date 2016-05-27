use iron::{Chain, Handler, IronResult, Request, Response, status};

use access_token::AccessToken;
use authentication::{AuthType, Flow, InteractiveAuth};
use config::Config;
use db::get_connection;
use middleware::{JsonRequest, UIAuth};
use modifier::SerializableResponse;
use user::User;

#[derive(Debug, Serialize)]
struct LoginResponse {
    pub access_token: String,
    pub home_server: String,
    pub user_id: String,
}


/// The /login endpoint.
pub struct Login;

impl Login {
    /// Create a `Login` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Login);

        chain.link_before(JsonRequest);

        let auth_request = UIAuth::new(
            InteractiveAuth::new(vec![Flow::new(vec![AuthType::Password])])
        );

        chain.link_before(auth_request);

        chain
    }
}

impl Handler for Login {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>().expect("UIAuth should ensure a user").clone();
        let connection = get_connection(request)?;
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

    #[test]
    fn valid_credentials() {
        let test = Test::new();

        assert!(test.post(
            "/_matrix/client/r0/register",
            r#"{"username": "carl", "password": "secret"}"#,
        ).status.is_success());

        let response = test.post(
            "/_matrix/client/r0/login",
            r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "secret"}}"#,
        );

        assert!(response.json().find("access_token").is_some());
        assert_eq!(response.json().find("home_server").unwrap().as_string().unwrap(), "ruma.test");
        assert_eq!(response.json().find("user_id").unwrap().as_string().unwrap(), "carl");
    }
}
