use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Plugin, Request};
use serde_json::Value;

use access_token::AccessToken;
use authentication::{AuthParams, InteractiveAuth, PasswordAuthParams};
use db::DB;
use error::APIError;
use user::User;

/// Handles access token authentication for all API endpoints that require it.
#[derive(Debug)]
pub struct AccessTokenAuth;

/// Handles Matrix's interactive authentication protocol for all API endpoints that require it.
#[derive(Debug)]
pub struct UIAuth {
    interactive_auth: InteractiveAuth,
}

impl UIAuth {
    /// Creates a new `UIAuth` from the given `InteractiveAuth`.
    pub fn new(interactive_auth: InteractiveAuth) -> Self {
        UIAuth {
            interactive_auth: interactive_auth,
        }
    }
}

impl BeforeMiddleware for AccessTokenAuth {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let connection = DB::from_request(request)?;
        let url = request.url.clone().into_generic_url();
        let mut query_pairs = url.query_pairs();

        if let Some((_, ref token)) = query_pairs.find(|&(ref key, _)| key == "access_token") {
            if let Ok(access_token) = AccessToken::find_valid_by_token(&connection, &token) {
                if let Ok(user) = User::find_by_access_token(&connection, &access_token) {
                    request.extensions.insert::<AccessToken>(access_token);
                    request.extensions.insert::<User>(user);

                    return Ok(());
                }
            }
        }

        Err(IronError::new(APIError::unauthorized(), APIError::unauthorized()))
    }
}

impl BeforeMiddleware for UIAuth {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let json = request
            .get::<bodyparser::Json>()
            .expect("bodyparser failed to parse")
            .expect("bodyparser did not find JSON in the body");

        if let Some(auth_json) = json.find("auth") {
            if is_m_login_password(auth_json) {
                if let Some((user, password)) = get_user_and_password(auth_json) {
                    let auth_params = AuthParams::Password(PasswordAuthParams {
                        password: password,
                        user: user,
                    });

                    let connection = DB::from_request(request)?;

                    if let Ok(user) =  auth_params.authenticate(&connection) {
                        request.extensions.insert::<User>(user);

                        return Ok(());
                    }
                }
            }
        }

        Err(IronError::new(APIError::unauthorized(), APIError::unauthorized()))
    }
}

fn get_user_and_password(json: &Value) -> Option<(String, String)> {
    let user = json.find("user").and_then(|user_json| user_json.as_str());
    let password = json.find("password").and_then(|password_json| password_json.as_str());

    match (user, password) {
        (Some(user), Some(password)) => Some((user.to_string(), password.to_string())),
        _ => None,
    }
}

fn is_m_login_password(json: &Value) -> bool {
    if let Some(type_string) = json.find("type").and_then(|type_json| type_json.as_str()) {
        return type_string == "m.login.password";
    }

    false
}
