use diesel::Connection;
use iron::typemap::Key;

pub struct DB;

impl Key for DB {
    type Value = Connection;
}
