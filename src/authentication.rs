//! User-interactive authentication.

use diesel::pg::PgConnection;
use iron::Response;
use iron::modifier::Modifier;
use serde::{Serialize, Serializer};

use error::{APIError, APIErrorCode};
use user::{User, load_user_with_plaintext_password};

/// A set of authorization flows the user can follow to authenticate a request.
#[derive(Debug, Serialize)]
pub struct InteractiveAuth {
    flows: Vec<Flow>,
}

impl InteractiveAuth {
    pub fn new(flows: Vec<Flow>) -> Self {
        InteractiveAuth {
            flows: flows,
        }
    }

    pub fn validate(&self, _params: &AuthParams) -> bool {
        true
    }

    pub fn with_auth_params(&self, _params: &AuthParams) -> &InteractiveAuth {
        self
    }
}

impl<'a> Modifier<Response> for &'a InteractiveAuth {
    fn modify(self, response: &mut Response) {
        response.status = Some(APIErrorCode::Forbidden.status_code());
        response.body = Some(Box::new(r#"{"flows":[{"stages":["m.login.dummy"]}]}"#));
    }
}

/// A list of `AuthType`s that satisfy authentication requirements.
#[derive(Debug, Serialize)]
pub struct Flow {
    #[serde(rename="stages")]
    auth_types: Vec<AuthType>,
}

impl Flow {
    pub fn new(auth_types: Vec<AuthType>) -> Self {
        Flow {
            auth_types: auth_types,
        }
    }
}

/// An individiual authentication mechanism to be used in a `Flow`.
#[derive(Clone, Debug)]
pub enum AuthType {
    /// m.login.password
    Password,
}

impl Serialize for AuthType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let value = match *self {
            AuthType::Password => "m.login.password",
        };

        serializer.serialize_str(value)
    }
}

/// Authentication parameters submitted by the user in a request.
#[derive(Clone, Debug)]
pub enum AuthParams {
    Password(PasswordAuthParams)
}

/// m.login.password reuqest parameters.
#[derive(Clone, Debug)]
pub struct PasswordAuthParams {
    /// The user's password as plaintext.
    pub password: String,
    /// The user's username.
    pub user: String,
}

impl AuthParams {
    pub fn authenticate(&self, connection: &PgConnection) -> Result<User, APIError> {
        let &AuthParams::Password(ref credentials) = self;

        load_user_with_plaintext_password(connection, &credentials.user, &credentials.password)
    }
}
