//! Database-related functionality.

use diesel::pg::PgConnection;
use iron::{Plugin, Request};
use iron::typemap::Key;
use persistent::Write;
use r2d2::{Pool, PooledConnection};
use r2d2_diesel::ConnectionManager;

use error::APIError;

/// An Iron plugin for attaching a database connection pool to an Iron request.
pub struct DB;

impl Key for DB {
    type Value = Pool<ConnectionManager<PgConnection>>;
}

/// Extract a database conection from the pool stored in the request.
pub fn get_connection(request: &mut Request)
    -> Result<PooledConnection<ConnectionManager<PgConnection>>, APIError>
{
    let mutex = try!(request.get::<Write<DB>>().map_err(APIError::from));
    let pool = try!(mutex.lock().map_err(|error| {
        APIError::unknown_from_string(format!("{}", error))
    }));
    pool.get().map_err(APIError::from)
}
