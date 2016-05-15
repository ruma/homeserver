use bodyparser;
use iron::{BeforeMiddleware, IronError, IronResult, Request};
use iron::typemap::Key;
use plugin::{Pluggable, Plugin};
use serde_json::from_value;

use authentication::{AuthParams, InteractiveAuth};
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
            if let Ok(ref auth_params) = from_value::<AuthParams>(auth_json.clone()) {
                let connection = try!(get_connection(request));
                return auth_params.authenticate(&connection);
            }
        }

        Err(APIError::unauthorized())
    }
}
