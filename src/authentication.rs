//! User-interactive authentication.

use serde::{Serialize, Serializer};

/// The full response body for a step in the authentication flow.
#[derive(Serialize)]
pub struct Response {
    flows: Vec<Flow>,
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
    Email(EmailAuthType),
    Password,
}

/// `AuthType`s related to email.
pub enum EmailAuthType {
    Identity,
}

impl Serialize for AuthType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let value = match self {
            &AuthType::Email(ref email_auth_type) => match email_auth_type {
                &EmailAuthType::Identity => "m.login.email.identity",
            },
            &AuthType::Password => "m.login.password",
        };

        serializer.serialize_str(value)
    }
}
