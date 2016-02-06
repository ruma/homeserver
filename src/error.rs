use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt::Error as FmtError;
use std::string::FromUtf8Error;

use base64::Base64Error;
use diesel::result::Error as DieselError;
use iron::{IronError, Response};
use iron::modifier::Modifier;
use iron::status::Status;
use persistent::PersistentError;
use r2d2::GetTimeout;
use serde_json;

#[derive(Clone, Debug, Serialize)]
pub struct APIError {
    #[serde(skip_serializing)]
    debug_message: Option<String>,
    errcode: APIErrorCode,
    error: String,
}

impl APIError {
    pub fn bad_json() -> APIError {
        APIError {
            debug_message: None,
            errcode: APIErrorCode::BadJson,
            error: "Invalid or missing key-value pairs in JSON.".to_owned(),
        }
    }

    pub fn not_json() -> APIError {
        APIError {
            debug_message: None,
            errcode: APIErrorCode::NotJson,
            error: "No JSON found in request body.".to_owned(),
        }
    }

    pub fn wrong_content_type() -> APIError {
        APIError {
            debug_message: None,
            errcode: APIErrorCode::NotJson,
            error: "Request's Content-Type header must be application/json.".to_owned(),
        }
    }

    pub fn unknown<E>(error: &E) -> APIError where E: Error {
        APIError {
            debug_message: Some(error.description().to_owned()),
            errcode: APIErrorCode::Unknown,
            error: "An unknown server-side error occurred.".to_owned(),
        }
    }

    pub fn unknown_from_string(message: String) -> APIError {
        APIError {
            debug_message: Some(message),
            errcode: APIErrorCode::Unknown,
            error: "An unknown server-side error occurred.".to_owned(),
        }
    }
}

impl Display for APIError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.error)
    }
}

impl Error for APIError {
    fn description(&self) -> &str {
        &self.error
    }
}

impl From<Base64Error> for APIError {
    fn from(error: Base64Error) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<DieselError> for APIError {
    fn from(error: DieselError) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<PersistentError> for APIError {
    fn from(error: PersistentError) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<GetTimeout> for APIError {
    fn from(error: GetTimeout) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<FromUtf8Error> for APIError {
    fn from(error: FromUtf8Error) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<APIError> for IronError {
    fn from(error: APIError) -> IronError {
        IronError::new(error.clone(), error)
    }
}

impl Modifier<Response> for APIError {
    fn modify(self, response: &mut Response) {
        response.status = Some(self.errcode.status_code());
        response.body = Some(Box::new(serde_json::to_string(&self).expect("APIError should always serialize")));
    }
}

#[derive(Clone, Debug, Serialize)]
pub enum APIErrorCode {
    BadJson,
    Forbidden,
    LimitExceeded,
    NotFound,
    NotJson,
    Unknown,
    UnknownToken,
}

impl APIErrorCode {
    pub fn status_code(&self) -> Status {
        match *self {
            APIErrorCode::BadJson => Status::UnprocessableEntity,
            APIErrorCode::Forbidden => Status::Forbidden,
            APIErrorCode::LimitExceeded => Status::TooManyRequests,
            APIErrorCode::NotFound => Status::NotFound,
            APIErrorCode::NotJson => Status::BadRequest,
            APIErrorCode::Unknown => Status::InternalServerError,
            APIErrorCode::UnknownToken => Status::Unauthorized,
        }
    }
}

pub struct CLIError {
    error: String,
}

impl CLIError {
    pub fn new<E>(error: E) -> CLIError where E: Into<String> {
        CLIError {
            error: error.into(),
        }
    }
}

impl<E> From<E> for CLIError where E: Error {
    fn from(error: E) -> CLIError {
        CLIError::new(error.description())
    }
}

impl Display for CLIError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.error)
    }
}
