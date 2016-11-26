//! Matrix filter.
use diesel::{
    ExpressionMethods,
    FilterDsl,
    LoadDsl,
    insert,
};
use diesel::pg::PgConnection;
use diesel::result::Error as DieselError;
use ruma_identifiers::UserId;

use error::ApiError;
use schema::filters;

/// A new Matrix filter, not yet saved.
#[derive(Debug, Insertable)]
#[table_name = "filters"]
pub struct NewFilter {
    /// The user's ID.
    pub user_id: UserId,
    /// The contents.
    pub content: String,
}

/// A new Matrix filter.
#[derive(AsChangeset, Debug, Clone, Identifiable, Queryable)]
#[table_name = "filters"]
pub struct Filter {
    /// Entry ID
    pub id: i64,
    /// The user's ID.
    pub user_id: UserId,
    /// The contents.
    pub content: String,
}


impl Filter {
    /// Creates a new `Filter`
    pub fn create(connection: &PgConnection, user_id: UserId, content: String)-> Result<i64, ApiError> {
        let new_filter = NewFilter {
            user_id: user_id,
            content: content,
        };

        let filter: Filter = insert(&new_filter)
            .into(filters::table)
            .get_result(connection)
            .map_err(ApiError::from)?;
        Ok(filter.id)
    }

    /// Return `Filter`'s for given `UserId` and `id`.
    pub fn find(connection: &PgConnection, user_id: UserId, id: i64) -> Result<Filter, ApiError> {
        let filter = filters::table
            .filter(filters::id.eq(id))
            .filter(filters::user_id.eq(user_id))
            .first(connection);

        match filter {
            Ok(filter) => Ok(filter),
            Err(DieselError::NotFound) => Err(ApiError::not_found("".to_string())),
            Err(err) => Err(ApiError::from(err)),
        }
    }
}
