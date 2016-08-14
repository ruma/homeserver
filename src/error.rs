//! Error types and conversions.

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fmt::Error as FmtError;
use std::io::Error as IoError;
use std::string::FromUtf8Error;

use base64::Base64Error;
use diesel::result::{TransactionError, Error as DieselError};
use iron::{IronError, Response};
use iron::modifier::Modifier;
use iron::status::Status;
use macaroons::error::Error as MacaroonsError;
use persistent::PersistentError;
use r2d2::GetTimeout;
use ruma_identifiers::Error as RumaIdentifiersError;
use serde::ser::{Serialize, Serializer};
use serde_json::{Error as SerdeJsonError, to_string};

/// A client-facing error.
#[derive(Clone, Debug, Serialize)]
pub struct APIError {
    errcode: APIErrorCode,
    error: String,
}

impl APIError {
    /// Create an error for invalid or incomplete JSON in request bodies.
    pub fn bad_json() -> APIError {
        APIError {
            errcode: APIErrorCode::BadJson,
            error: "Invalid or missing key-value pairs in JSON.".to_string(),
        }
    }

    /// Create an error for endpoints where guest accounts are not supported.
    pub fn guest_forbidden() -> APIError {
        APIError {
            errcode: APIErrorCode::GuestAccessForbidden,
            error: "Guest accounts are forbidden.".to_string(),
        }
    }

    /// Create an error for requests that do not map to a resource.
    pub fn not_found() -> APIError {
        APIError {
            errcode: APIErrorCode::NotFound,
            error: "No resource was found for this request.".to_string(),
        }
    }

    /// Create an error for requests without JSON bodies.
    pub fn not_json() -> APIError {
        APIError {
            errcode: APIErrorCode::NotJson,
            error: "No JSON found in request body.".to_string(),
        }
    }

    /// Create an error for requests that are not marked as containing JSON.
    pub fn wrong_content_type() -> APIError {
        APIError {
            errcode: APIErrorCode::NotJson,
            error: "Request's Content-Type header must be application/json.".to_string(),
        }
    }

    /// Create an error for requests that did not provide required authentication parameters.
    pub fn unauthorized() -> APIError {
        APIError {
            errcode: APIErrorCode::Forbidden,
            error: "Authentication is required.".to_string(),
        }
    }

    /// Create a generic error for anything not specifically covered by the Matrix spec.
    pub fn unknown<D: Debug + ?Sized>(error: &D) -> APIError {
        debug!("API error: {:?}", error);

        APIError {
            errcode: APIErrorCode::Unknown,
            error: "An unknown server-side error occurred.".to_string(),
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

impl From<IoError> for APIError {
    fn from(error: IoError) -> APIError {
        APIError::unknown(&error)
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

impl<T> From<TransactionError<T>> for APIError where T: Error {
    fn from(error: TransactionError<T>) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<MacaroonsError> for APIError {
    fn from(error: MacaroonsError) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<PersistentError> for APIError {
    fn from(error: PersistentError) -> APIError {
        APIError::unknown(&error)
    }
}

impl From<RumaIdentifiersError> for APIError {
    fn from(error: RumaIdentifiersError) -> APIError {
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

impl From<SerdeJsonError> for APIError {
    fn from(error: SerdeJsonError) -> APIError {
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
        response.body = Some(Box::new(to_string(&self).expect("APIError should always serialize")));
    }
}

/// The error code for a client-facing error.
#[derive(Clone, Debug)]
pub enum APIErrorCode {
    /// The request contained valid JSON, but it was malformed in some way,
    /// e.g. missing required keys, invalid values for keys.
    BadJson,
    /// Forbidden access, e.g. joining a room without permission, failed login.
    Forbidden,
    /// Guests are not allowed to perform the requested operation.
    GuestAccessForbidden,
    /// Too many requests have been sent in a short period of time. Wait a while then try again.
    LimitExceeded,
    /// No resource was found for this request.
    NotFound,
    /// Request did not contain valid JSON.
    NotJson,
    /// Errors not fitting into another category.
    Unknown,
    /// The access token specified was not recognised.
    UnknownToken,
}

impl APIErrorCode {
    /// The HTTP status code that should be used to represent the `APIErrorCode`.
    pub fn status_code(&self) -> Status {
        match *self {
            APIErrorCode::BadJson => Status::UnprocessableEntity,
            APIErrorCode::Forbidden => Status::Forbidden,
            APIErrorCode::GuestAccessForbidden => Status::Forbidden,
            APIErrorCode::LimitExceeded => Status::TooManyRequests,
            APIErrorCode::NotFound => Status::NotFound,
            APIErrorCode::NotJson => Status::BadRequest,
            APIErrorCode::Unknown => Status::InternalServerError,
            APIErrorCode::UnknownToken => Status::Unauthorized,
        }
    }
}

impl Serialize for APIErrorCode {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let value = match *self {
            APIErrorCode::BadJson => "M_BAD_JSON",
            APIErrorCode::Forbidden => "M_FORBIDDEN",
            APIErrorCode::GuestAccessForbidden => "M_GUEST_ACCESS_FORBIDDEN",
            APIErrorCode::LimitExceeded => "M_LIMIT_EXCEEDED",
            APIErrorCode::NotFound => "M_NOT_FOUND",
            APIErrorCode::NotJson => "M_NOT_JSON",
            APIErrorCode::Unknown => "M_UNKNOWN",
            APIErrorCode::UnknownToken => "M_UNKNOWN_TOKEN",
        };

        serializer.serialize_str(value)
    }
}

/// An operator-facing error.
pub struct CLIError {
    error: String,
}

impl CLIError {
    /// Create a new `CLIError` from any `Error` type.
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
