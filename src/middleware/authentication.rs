use std::convert::TryFrom;

use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Plugin, Request};
use ruma_identifiers::UserId;
use serde_json::Value;

use authentication::{AuthParams, InteractiveAuth, PasswordAuthParams};
use config::Config;
use db::DB;
use error::ApiError;
use models::access_token::AccessToken;
use models::user::User;

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

        Err(IronError::new(ApiError::unauthorized(None), ApiError::unauthorized(None)))
    }
}

impl BeforeMiddleware for UIAuth {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let json = request
            .get::<bodyparser::Json>()
            .expect("bodyparser failed to parse")
            .expect("bodyparser did not find JSON in the body");
        let config = Config::from_request(request)?;

        if let Some(auth_json) = json.find("auth") {
            if is_m_login_password(auth_json) {
                if let Ok((user_id, password)) = get_user_id_and_password(auth_json, &config) {
                    let auth_params = AuthParams::Password(PasswordAuthParams {
                        password: password,
                        user_id: user_id,
                    });

                    let connection = DB::from_request(request)?;

                    if let Ok(user) =  auth_params.authenticate(&connection) {
                        request.extensions.insert::<User>(user);

                        return Ok(());
                    }
                }
            }
        }

        Err(IronError::new(ApiError::unauthorized(None), ApiError::unauthorized(None)))
    }
}

fn get_user_id_and_password(json: &Value, config: &Config) -> Result<(UserId, String), ()> {
    let username = json.find("user").and_then(|username_json| username_json.as_str());
    let password = json.find("password").and_then(|password_json| password_json.as_str());

    match (username, password) {
        (Some(username), Some(password)) => {
            match UserId::try_from(username) {
                Ok(user_id) => Ok((user_id, password.to_string())),
                Err(_) => match UserId::try_from(&format!("@{}:{}", username, &config.domain)) {
                    Ok(user_id) => Ok((user_id, password.to_string())),
                    Err(_) => Err(()),
                },
            }
        }
        _ => Err(()),
    }
}

fn is_m_login_password(json: &Value) -> bool {
    if let Some(type_string) = json.find("type").and_then(|type_json| type_json.as_str()) {
        return type_string == "m.login.password";
    }

    false
}
