//! Iron middleware.
use iron::Chain;

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

/// MiddlewareChain
pub trait MiddlewareChain {
    /// Create a `MiddlewareChain` with all necessary middleware.
    fn chain() -> Chain;
}
