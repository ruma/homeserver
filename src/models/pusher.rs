//! Matrix pusher.

use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::result::Error as DieselError;
use ruma_identifiers::UserId;

use error::ApiError;
use schema::pushers;

/// Data need for kind is http.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PusherData {
    /// Required if kind is http. The URL to use to send notifications to.
    pub url: Option<String>
}

/// Options for updating pusher
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PusherOptions {
    /// The preferred language for receiving notifications (e.g. 'en' or 'en-US')
    pub lang: String,
    /// The kind of pusher. "http" is a pusher that sends HTTP pokes.
    pub kind: String,
    /// A dictionary of information for the pusher implementation itself.
    pub data: PusherData,
    /// A string that will allow the user to identify what device owns this pusher.
    pub device_display_name: String,
    /// This is a reverse-DNS style identifier for the application. Max length, 64 chars.
    pub app_id: String,
    /// This string determines which set of device specific rules this pusher executes.
    pub profile_tag: Option<String>,
    /// This is a unique identifier for this pusher. Max length, 512 bytes.
    pub pushkey: String,
    /// A string that will allow the user to identify what application owns this pusher.
    pub app_display_name: String,
    /// If true, the homeserver should add another pusher with the given pushkey and
    /// App ID in addition to any others with different user IDs.
    /// Otherwise, the homeserver must remove any other pushers
    /// with the same App ID and pushkey for different users.
    ///
    /// The default is false.
    #[serde(default = "default_append")]
    #[serde(skip_serializing_if = "is_false")]
    pub append: bool,
}

impl PusherOptions {
    /// Check for url when kind is http.
    pub fn is_valid(&self) -> bool {
        if self.kind == "http" {
            return self.data.url.is_some()
        }
        true
    }
}

impl From<Pusher> for PusherOptions {
    fn from(pusher: Pusher) -> PusherOptions {
        PusherOptions {
            lang: pusher.lang,
            kind: pusher.kind,
            data: PusherData {
                url: pusher.url,
            },
            device_display_name: pusher.device_display_name,
            app_id: pusher.app_id,
            profile_tag: pusher.profile_tag,
            pushkey: pusher.pushkey,
            app_display_name: pusher.app_display_name,
            append: false,
        }
    }
}

fn default_append() -> bool {
    false
}

fn is_false(test: &bool) -> bool {
    !test
}

/// A matrix pusher.
#[derive(AsChangeset, Clone, Debug, Identifiable, Insertable, Queryable)]
#[table_name = "pushers"]
#[primary_key(user_id, app_id)]
pub struct Pusher {
    /// The user's ID.
    pub user_id: UserId,
    /// The preferred language for receiving notifications (e.g. 'en' or 'en-US')
    pub lang: String,
    /// The kind of pusher. "http" is a pusher that sends HTTP pokes.
    pub kind: String,
    /// Required if kind is http. The URL to use to send notifications to.
    pub url: Option<String>,
    /// A string that will allow the user to identify what device owns this pusher.
    pub device_display_name: String,
    /// This is a reverse-DNS style identifier for the application. Max length, 64 chars.
    pub app_id: String,
    /// This string determines which set of device specific rules this pusher executes.
    pub profile_tag: Option<String>,
    /// This is a unique identifier for this pusher. Max length, 512 bytes.
    pub pushkey: String,
    /// A string that will allow the user to identify what application owns this pusher.
    pub app_display_name: String,
}

impl Pusher {
    /// Update or Create a `Pusher` entry based on `PusherOptions`.
    pub fn upsert(
        connection: &PgConnection,
        user_id: &UserId,
        options: &PusherOptions
    ) -> Result<Pusher, ApiError> {
        connection.transaction::<Pusher, ApiError, _>(|| {
            if !options.is_valid() {
                return Err(ApiError::bad_json("If kind is http, data.url shouldn't be null.".to_string()))
            }
            match options.append {
                true => {
                    let pusher = Pusher::find(
                        connection,
                        &user_id,
                        &options.app_id
                    )?;
                    match pusher {
                        Some(mut pusher) => {
                            pusher.update(connection, options.clone())?;
                            return Ok(pusher)
                        },
                        None => (),
                    }
                },
                false => {
                    Pusher::delete_by_app_id_and_pushkey(
                        connection,
                        &options.app_id,
                        &options.pushkey
                    )?;
                },
            }
            Ok(Pusher::create(connection, user_id.clone(), options.clone())?)
        }).map_err(ApiError::from)
    }

    /// Delete a `Pusher`
    pub fn delete(
        connection: &PgConnection,
        user_id: &UserId,
        app_id: &str
    ) -> Result<(), ApiError> {
        let pusher = pushers::table.find((user_id, &app_id));
        diesel::delete(pusher).execute(connection)?;
        Ok(())
    }

    /// Update a `Pusher`
    fn update(
        &mut self,
        connection: &PgConnection,
        options: PusherOptions
    ) -> Result<(), ApiError> {
        self.kind = options.kind;
        self.app_display_name = options.app_display_name;
        self.lang = options.lang;
        self.device_display_name = options.device_display_name;
        self.profile_tag = options.profile_tag;
        self.url = options.data.url;

        match self.save_changes::<Pusher>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Create a new `Pusher`.
    fn create(
        connection: &PgConnection,
        user_id: UserId,
        options: PusherOptions
    ) -> Result<Pusher, ApiError> {
        let new_pusher = Pusher {
            user_id: user_id,
            lang: options.lang,
            kind: options.kind,
            device_display_name: options.device_display_name,
            app_id: options.app_id,
            profile_tag: options.profile_tag,
            pushkey: options.pushkey,
            app_display_name: options.app_display_name,
            url: options.data.url,
        };

        diesel::insert_into(pushers::table)
            .values(&new_pusher)
            .get_result(connection)
            .map_err(ApiError::from)
    }

    /// Return a `Pusher` by primary keys.
    pub fn find(
        connection: &PgConnection,
        user_id: &UserId,
        app_id: &str
    ) -> Result<Option<Pusher>, ApiError> {
        let pusher = pushers::table.find((user_id, app_id)).get_result(connection);

        match pusher {
            Ok(pusher) => Ok(Some(pusher)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Delete all `Pusher`'s for given `app_id` and `pushkey`.
    pub fn delete_by_app_id_and_pushkey(
        connection: &PgConnection,
        app_id: &str,
        pushkey: &str
    ) -> Result<(), ApiError> {
        let pushers = pushers::table
            .filter(pushers::app_id.eq(app_id))
            .filter(pushers::pushkey.eq(pushkey));
        diesel::delete(pushers).execute(connection)?;
        Ok(())
    }

    /// Return all `Pusher`'s for given `UserId`.
    pub fn find_by_uid(
        connection: &PgConnection,
        user_id: &UserId
    ) -> Result<Vec<Pusher>, ApiError> {
        pushers::table
            .filter(pushers::user_id.eq(user_id))
            .get_results(connection).map_err(ApiError::from)
    }
}
