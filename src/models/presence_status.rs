//! Storage and querying of presence status.

use chrono::{Duration, NaiveDateTime, NaiveDate, UTC};
use diesel::{
    insert,
    Connection,
    ExecuteDsl,
    ExpressionMethods,
    FindDsl,
    FilterDsl,
    LoadDsl,
    SaveChangesDsl,
};
use diesel::expression::dsl::any;
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::Error as DieselError;
use ruma_events::presence::PresenceState;
use ruma_identifiers::{UserId, EventId};

use error::ApiError;
use schema::presence_status;

/// A Matrix presence status, not saved yet.
#[derive(Debug, Clone, Insertable)]
#[table_name = "presence_status"]
pub struct NewPresenceStatus {
    /// The user's ID.
    pub user_id: UserId,
    /// The unique event ID.
    pub event_id: EventId,
    /// The current presence state.
    pub presence: String,
    /// A possible status message from the user.
    pub status_msg: Option<String>,
    /// Timestamp of the last update.
    pub updated_at: PgTimestamp,
}

/// A Matrix presence status.
#[derive(Debug, Clone, Queryable, Identifiable, AsChangeset)]
#[table_name = "presence_status"]
#[primary_key(user_id)]
pub struct PresenceStatus {
    /// The user's ID.
    pub user_id: UserId,
    /// The unique event ID.
    pub event_id: EventId,
    /// The current presence state.
    pub presence: String,
    /// A possible status message from the user.
    pub status_msg: Option<String>,
    /// Timestamp of the last update.
    pub updated_at: PgTimestamp,
}

/// Return current time in milliseconds
pub fn get_now() -> i64 {
    let now = UTC::now().naive_utc();
    get_milliseconds(now)
}

/// Return `time` in milliseconds with a same epoch as PostgreSQL.
pub fn get_milliseconds(time: NaiveDateTime) -> i64 {
    let epoch = NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);
    let duration: Duration = time.signed_duration_since(epoch);
    duration.num_milliseconds()
}

impl PresenceStatus {
    /// Update or insert a presence status entry.
    pub fn upsert(
        connection: &PgConnection,
        homeserver_domain: &str,
        user_id: &UserId,
        presence: Option<PresenceState>,
        status_msg: Option<String>
    ) -> Result<(), ApiError> {
        let event_id = &EventId::new(&homeserver_domain).map_err(ApiError::from)?;

        connection.transaction::<(), ApiError, _>(|| {
            let status = PresenceStatus::find_by_uid(connection, user_id)?;
            let presence = match presence {
                Some(presence) => presence.to_string(),
                None => match status {
                    Some(ref status) => status.presence.clone(),
                    None => "offline".to_string(),
                }
            };

            match status {
                Some(mut status) => status.update(connection, presence, status_msg, event_id),
                None => PresenceStatus::create(connection, user_id, presence, status_msg, event_id),
            }
        }).map_err(ApiError::from)
    }

    /// Update a presence status entry.
    fn update(
        &mut self,
        connection: &PgConnection,
        presence: String,
        status_msg: Option<String>,
        event_id: &EventId
    ) -> Result<(), ApiError> {
        self.presence = presence;
        self.status_msg = status_msg;
        self.event_id = event_id.clone();
        self.updated_at = PgTimestamp(get_now());

        match self.save_changes::<PresenceStatus>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Create a presence status entry.
    fn create(
        connection: &PgConnection,
        user_id: &UserId,
        presence: String,
        status_msg: Option<String>,
        event_id: &EventId
    ) -> Result<(), ApiError> {
        let new_status = NewPresenceStatus {
            user_id: user_id.clone(),
            event_id: event_id.clone(),
            presence: presence,
            status_msg: status_msg,
            updated_at: PgTimestamp(get_now()),
        };
        insert(&new_status)
            .into(presence_status::table)
            .execute(connection)
            .map_err(ApiError::from)?;
        Ok(())
    }

    /// Return `PresenceStatus` for given `UserId`.
    pub fn find_by_uid(
        connection: &PgConnection,
        user_id: &UserId
    ) -> Result<Option<PresenceStatus>, ApiError> {
        let status = presence_status::table.find(user_id).first(connection);

        match status{
            Ok(status) => Ok(Some(status)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Get status entries for a list of `UserId`'s which were updated after a
    /// specific point in time.
    pub fn get_users(
        connection: &PgConnection,
        users: &Vec<UserId>,
        since: Option<i64>,
    ) -> Result<Vec<PresenceStatus>, ApiError> {
        match since {
            Some(since) => {
                let time = PgTimestamp(since);

                presence_status::table
                    .filter(presence_status::user_id.eq(any(users)))
                    .filter(presence_status::updated_at.gt(time))
                    .get_results(connection)
                    .map_err(ApiError::from)
            },
            None => {
                presence_status::table
                    .filter(presence_status::user_id.eq(any(users)))
                    .get_results(connection)
                    .map_err(ApiError::from)
            }
        }
    }
}
