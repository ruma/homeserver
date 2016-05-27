use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Request};
use iron::typemap::Key;
use plugin::{Pluggable, Plugin};
use serde_json::Value;

use authentication::{AuthParams, InteractiveAuth, PasswordAuthParams};
use db::get_connection;
use error::APIError;
use user::User;

/// Handles Matrix's interactive authentication protocol for all API endpoints that require it.
#[derive(Debug)]
pub struct UIAuth {
    interactive_auth: InteractiveAuth,
}

#[derive(Debug)]
pub struct AuthRequest;

impl UIAuth {
    pub fn new(interactive_auth: InteractiveAuth) -> Self {
        UIAuth {
            interactive_auth: interactive_auth,
        }
    }
}

impl BeforeMiddleware for UIAuth {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        match request.get::<AuthRequest>() {
            Ok(_) => Ok(()),
            Err(error) => Err(IronError::new(error.clone(), error)),
        }
    }
}

impl Key for AuthRequest {
    type Value = User;
}

impl<'a, 'b> Plugin<Request<'a, 'b>> for AuthRequest {
    type Error = APIError;

    fn eval(request: &mut Request) -> Result<Self::Value, Self::Error> {
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

                    let connection = get_connection(request)?;

                    return auth_params.authenticate(&connection);
                }
            }
        }

        Err(APIError::unauthorized())
    }
}

fn get_user_and_password(json: &Value) -> Option<(String, String)> {
    let user = json.find("user").and_then(|user_json| user_json.as_string());
    let password = json.find("password").and_then(|password_json| password_json.as_string());

    match (user, password) {
        (Some(user), Some(password)) => Some((user.to_string(), password.to_string())),
        _ => None,
    }
}

fn is_m_login_password(json: &Value) -> bool {
    if let Some(type_string) = json.find("type").and_then(|type_json| type_json.as_string()) {
        return type_string == "m.login.password";
    }

    false
}
