//! User-interactive authentication.

use iron::Response;
use iron::modifier::Modifier;
use serde::{Serialize, Serializer};

use error::APIErrorCode;

/// A set of authorization flows the user can follow to authenticate a request.
#[derive(Serialize)]
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
#[derive(Serialize)]
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
pub enum AuthType {
    Dummy,
    Email(EmailAuthType),
    OAuth2,
    Password,
    ReCaptcha,
    Token,
}

/// `AuthType`s related to email.
pub enum EmailAuthType {
    Identity,
}

impl Serialize for AuthType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let value = match *self {
            AuthType::Dummy => "m.login.dummy",
            AuthType::Email(ref email_auth_type) => match *email_auth_type {
                EmailAuthType::Identity => "m.login.email.identity",
            },
            AuthType::OAuth2 => "m.login.oauth2",
            AuthType::Password => "m.login.password",
            AuthType::ReCaptcha => "m.login.recaptcha",
            AuthType::Token => "m.login.token",
        };

        serializer.serialize_str(value)
    }
}

/// Authentication parameters submitted by the user in a request.
#[derive(Deserialize)]
pub enum AuthParams {
    Dummy
}
