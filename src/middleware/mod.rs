//! Iron middleware.
use iron::Chain;

mod authentication;
mod json;
mod path_params;
mod response_headers;

pub use self::authentication::{AccessTokenAuth, UIAuth};
pub use self::json::JsonRequest;
pub use self::path_params::{
    DataTypeParam, EventTypeParam, FilterIdParam, RoomAliasIdParam, RoomIdOrAliasParam,
    RoomIdParam, TagParam, TransactionIdParam, UserIdParam,
};
pub use self::response_headers::ResponseHeaders;

/// `middleware_chain!(JoinRoom, []);`
#[macro_export]
macro_rules! middleware_chain {
    ($chain:ident) => {chain_impl!($chain, []);};
    ($chain:ident, [$($middleware:expr),*]) => {
        impl MiddlewareChain for $chain {
            /// Create a `$chain` with all necessary middleware.
            fn chain() -> Chain {
                let mut chain = Chain::new($chain);
                $(chain.link_before($middleware);)*

                chain
            }
        }
    };
}

/// `MiddlewareChain` ensures that endpoints have a chain function.
pub trait MiddlewareChain {
    /// Create a `MiddlewareChain` with all necessary middleware.
    fn chain() -> Chain;
}
