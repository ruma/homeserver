//! Iron middleware.

mod authentication;
mod cors;
mod json;
mod path_params;

pub use self::authentication::{AccessTokenAuth, UIAuth};
pub use self::cors::Cors;
pub use self::json::JsonRequest;
pub use self::path_params::{
    DataTypeParam,
    EventTypeParam,
    UserIdParam,
    RoomIdParam,
    RoomAliasIdParam,
    TransactionIdParam,
};
