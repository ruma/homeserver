//! Error types and conversions.

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fmt::Error as FmtError;
use std::io::Error as IoError;
use std::string::FromUtf8Error;
use std::sync::PoisonError;

use argon2rs::verifier::DecodeError;
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
pub struct ApiError {
    errcode: ApiErrorCode,
    error: String,
}

/// The error code for a client-facing error.
#[derive(Clone, Debug)]
pub enum ApiErrorCode {
    /// The requested room alias is already taken.
    AliasTaken,
    /// Request contained an event that was not valid input for the requested API.
    BadEvent,
    /// The request contained valid JSON, but it was malformed in some way,
    /// e.g. missing required keys, invalid values for keys.
    BadJson,
    /// Forbidden access, e.g. joining a room without permission, failed login.
    Forbidden,
    /// Guests are not allowed to perform the requested operation.
    GuestAccessForbidden,
    /// Too many requests have been sent in a short period of time. Wait a while then try again.
    LimitExceeded,
    /// A required input parameter was not supplied, e.g. query string or URL path-based parameter.
    MissingParam,
    /// No resource was found for this request.
    NotFound,
    /// Request did not contain valid JSON.
    NotJson,
    /// Ruma does not implement the requested API.
    Unimplemented,
    /// Errors not fitting into another category.
    Unknown,
    /// The access token specified was not recognised.
    UnknownToken,
}

/// An operator-facing error.
pub struct CliError {
    error: String,
}

/// Extensions for `Result` to make handling Ruma API errors easier.
pub trait MapApiError {
    /// The type contained in a successful result.
    type Output;
    /// The initial error type contained in an unsuccessful result.
    ///
    /// This will be converted to `ApiError`.
    type Error: Debug;

    /// Similar to `map_err`, but prints the original error to the debug log and must always
    /// return an `ApiError`.
    fn map_api_err<O>(self, op: O) -> Result<Self::Output, ApiError>
    where O: FnOnce(Self::Error) -> ApiError;
}

impl ApiError {
    /// Create an error for requests that try to create a room alias that is already taken.
    pub fn alias_taken(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::AliasTaken,
            error: message.unwrap_or("Alias already taken.").to_string(),
        }
    }

    /// Create an error for invalid or incomplete input to event creation API endpoints.
    pub fn bad_event(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::BadEvent,
            error: message.unwrap_or("Invalid event data.").to_string(),
        }
    }

    /// Create an error for invalid or incomplete JSON in request bodies.
    pub fn bad_json(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::BadJson,
            error: message.unwrap_or("Invalid or missing key-value pairs in JSON.").to_string(),
        }
    }

    /// Create an error for endpoints where guest accounts are not supported.
    pub fn guest_forbidden(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::GuestAccessForbidden,
            error: message.unwrap_or("Guest accounts are forbidden.").to_string(),
        }
    }

    /// Create an error for requests missing a value for a required parameter.
    pub fn missing_param(param_name: &str) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::MissingParam,
            error: format!("Missing value for required parameter: {}.", param_name),
        }
    }

    /// Create an error for requests that do not map to a resource.
    pub fn not_found(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::NotFound,
            error: message.unwrap_or("No resource was found for this request.").to_string(),
        }
    }

    /// Create an error for requests without JSON bodies.
    pub fn not_json(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::NotJson,
            error: message.unwrap_or("No JSON found in request body.").to_string(),
        }
    }

    /// Create an error for requests that are not marked as containing JSON.
    pub fn wrong_content_type(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::NotJson,
            error: message.unwrap_or(
                "Request's Content-Type header must be application/json."
            ).to_string(),
        }
    }

    /// Create an error for requests that did not provide required authentication parameters.
    pub fn unauthorized(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::Forbidden,
            error: message.unwrap_or("Authentication is required.").to_string(),
        }
    }

    /// Create an error for Matrix APIs that Ruma intentionally does not implement.
    pub fn unimplemented(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::Unimplemented,
            error: message.unwrap_or("The homeserver does not implement this API.").to_string(),
        }
    }

    /// Create a generic error for anything not specifically covered by the Matrix spec.
    pub fn unknown(message: Option<&str>) -> ApiError {
        ApiError {
            errcode: ApiErrorCode::Unknown,
            error: message.unwrap_or("An unknown server-side error occurred.").to_string(),
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.error)
    }
}

impl Error for ApiError {
    fn description(&self) -> &str {
        &self.error
    }
}

impl From<IoError> for ApiError {
    fn from(error: IoError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<DecodeError> for ApiError {
    fn from(error: DecodeError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}


impl From<Base64Error> for ApiError {
    fn from(error: Base64Error) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<DieselError> for ApiError {
    fn from(error: DieselError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl<T> From<TransactionError<T>> for ApiError where T: Error {
    fn from(error: TransactionError<T>) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<MacaroonsError> for ApiError {
    fn from(error: MacaroonsError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<PersistentError> for ApiError {
    fn from(error: PersistentError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<RumaIdentifiersError> for ApiError {
    fn from(error: RumaIdentifiersError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<GetTimeout> for ApiError {
    fn from(error: GetTimeout) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<FromUtf8Error> for ApiError {
    fn from(error: FromUtf8Error) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl<T> From<PoisonError<T>> for ApiError {
    fn from(error: PoisonError<T>) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<SerdeJsonError> for ApiError {
    fn from(error: SerdeJsonError) -> ApiError {
        debug!("Converting to ApiError from: {:?}", error);

        ApiError::unknown(None)
    }
}

impl From<ApiError> for IronError {
    fn from(error: ApiError) -> IronError {
        IronError::new(error.clone(), error)
    }
}

impl Modifier<Response> for ApiError {
    fn modify(self, response: &mut Response) {
        response.status = Some(self.errcode.status_code());
        response.body = Some(Box::new(to_string(&self).expect("ApiError should always serialize")));
    }
}

impl ApiErrorCode {
    /// The HTTP status code that should be used to represent the `ApiErrorCode`.
    pub fn status_code(&self) -> Status {
        match *self {
            ApiErrorCode::AliasTaken => Status::Conflict,
            ApiErrorCode::BadEvent => Status::UnprocessableEntity,
            ApiErrorCode::BadJson => Status::UnprocessableEntity,
            ApiErrorCode::Forbidden => Status::Forbidden,
            ApiErrorCode::GuestAccessForbidden => Status::Forbidden,
            ApiErrorCode::LimitExceeded => Status::TooManyRequests,
            ApiErrorCode::MissingParam => Status::BadRequest,
            ApiErrorCode::NotFound => Status::NotFound,
            ApiErrorCode::NotJson => Status::BadRequest,
            ApiErrorCode::Unimplemented => Status::NotFound,
            ApiErrorCode::Unknown => Status::InternalServerError,
            ApiErrorCode::UnknownToken => Status::Unauthorized,
        }
    }
}

impl Serialize for ApiErrorCode {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        let value = match *self {
            ApiErrorCode::AliasTaken => "IO_RUMA_ALIAS_TAKEN",
            ApiErrorCode::BadEvent => "IO_RUMA_BAD_EVENT",
            ApiErrorCode::BadJson => "M_BAD_JSON",
            ApiErrorCode::Forbidden => "M_FORBIDDEN",
            ApiErrorCode::GuestAccessForbidden => "M_GUEST_ACCESS_FORBIDDEN",
            ApiErrorCode::LimitExceeded => "M_LIMIT_EXCEEDED",
            ApiErrorCode::MissingParam => "M_MISSING_PARAM",
            ApiErrorCode::NotFound => "M_NOT_FOUND",
            ApiErrorCode::NotJson => "M_NOT_JSON",
            ApiErrorCode::Unimplemented => "IO_RUMA_UNIMPLEMENTED",
            ApiErrorCode::Unknown => "M_UNKNOWN",
            ApiErrorCode::UnknownToken => "M_UNKNOWN_TOKEN",
        };

        serializer.serialize_str(value)
    }
}

impl CliError {
    /// Create a new `CliError` from any `Error` type.
    pub fn new<E>(error: E) -> CliError where E: Into<String> {
        CliError {
            error: error.into(),
        }
    }
}

impl<E> From<E> for CliError where E: Error {
    fn from(error: E) -> CliError {
        CliError::new(error.description())
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.error)
    }
}

impl<T, E> MapApiError for Result<T, E> where E: Debug {
    type Output = T;
    type Error = E;

    #[inline]
    fn map_api_err<O>(self, op: O) -> Result<T, ApiError> where O: FnOnce(E) -> ApiError {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                debug!("Converting to ApiError from: {:?}", e);

                Err(op(e))
            }
        }
    }
}
