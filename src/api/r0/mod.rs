//! API endpoints for the 0.x.x version of the Matrix spec.

pub use self::account::AccountPassword;
pub use self::login::Login;
pub use self::logout::Logout;
pub use self::registration::Register;
pub use self::versions::Versions;

mod account;
mod login;
mod logout;
mod registration;
mod versions;
