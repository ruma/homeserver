use diesel::pg::data_types::PgTimestamp;

use schema::users;

#[derive(Debug, Queryable)]
pub struct User {
    pub id: String,
    pub password_hash: String,
    pub created_at: PgTimestamp,
    pub updated_at: PgTimestamp,
}

#[derive(Debug)]
#[insertable_into(users)]
pub struct NewUser {
    pub id: String,
    pub password_hash: String,
}

