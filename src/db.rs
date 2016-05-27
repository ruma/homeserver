//! Database-related functionality.

use diesel::pg::PgConnection;
use iron::{Plugin, Request};
use iron::typemap::Key;
use persistent::Write;
use r2d2::{Config as R2D2Config, InitializationError, Pool, PooledConnection};
use r2d2_diesel::{ConnectionManager, Error as R2D2DieselError};

use error::APIError;

/// An Iron plugin for attaching a database connection pool to an Iron request.
pub struct DB;

impl DB {
    /// Creates a connection pool for the PostgreSQL database at the given URL.
    pub fn create_connection_pool(
        r2d2_config: R2D2Config<PgConnection, R2D2DieselError>,
        postgres_url: &str
    ) -> Result<Pool<ConnectionManager<PgConnection>>, InitializationError> {
        let connection_manager = ConnectionManager::new(postgres_url);

        Pool::new(r2d2_config, connection_manager)
    }

    /// Extract a database conection from the pool stored in the request.
    pub fn from_request(request: &mut Request)
        -> Result<PooledConnection<ConnectionManager<PgConnection>>, APIError>
    {
        let mutex = request.get::<Write<DB>>().map_err(APIError::from)?;
        let pool = mutex.lock().map_err(|error| {
            APIError::unknown_from_string(format!("{}", error))
        })?;
        pool.get().map_err(APIError::from)
    }
}

impl Key for DB {
    type Value = Pool<ConnectionManager<PgConnection>>;
}
