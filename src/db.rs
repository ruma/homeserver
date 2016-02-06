use diesel::pg::PgConnection;
use iron::typemap::Key;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

pub struct DB;

impl Key for DB {
    type Value = Pool<ConnectionManager<PgConnection>>;
}
