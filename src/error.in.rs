use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt::Error as FmtError;
use std::io::Error as IoError;

use iron::Response;
use iron::modifier::Modifier;
use iron::status::Status;
use serde_json;

#[derive(Clone, Debug, Serialize)]
pub struct APIError {
    errcode: APIErrorCode,
    error: String,
}

impl APIError {
    pub fn bad_json(error: &Error) -> Self {
        APIError {
            errcode: APIErrorCode::BadJson,
            error: error.description().to_owned(),
        }
    }

    pub fn not_json() -> Self {
        APIError {
            errcode: APIErrorCode::NotJson,
            error: "No JSON found in request body.".to_owned(),
        }
    }

    pub fn wrong_content_type() -> Self {
        APIError {
            errcode: APIErrorCode::NotJson,
            error: "Request's Content-Type header must be application/json.".to_owned(),
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
    UnknownToken,
}

impl APIErrorCode {
    pub fn status_code(&self) -> Status {
        match *self {
            APIErrorCode::BadJson => Status::BadRequest,
            APIErrorCode::Forbidden => Status::Forbidden,
            APIErrorCode::LimitExceeded => Status::TooManyRequests,
            APIErrorCode::NotFound => Status::NotFound,
            APIErrorCode::NotJson => Status::BadRequest,
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

impl From<IoError> for CLIError {
    fn from(error: IoError) -> CLIError {
        CLIError::new(error.description())
    }
}

impl Display for CLIError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.error)
    }
}
