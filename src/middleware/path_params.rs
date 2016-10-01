use std::convert::TryFrom;
use std::convert::From;
use std::error::Error;

use iron::{BeforeMiddleware, IronResult, IronError, Request};
use iron::typemap::Key;
use router::Router;
use ruma_events::EventType;
use ruma_identifiers::{
    UserId,
    RoomAliasId,
    RoomId,
};

use config::Config;
use error::{ApiError, MapApiError};

/// Extracts a `RoomId` from the URL path parameter `room_id`.
pub struct RoomIdParam;

impl Key for RoomIdParam {
    type Value = RoomId;
}

impl BeforeMiddleware for RoomIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();
        let room_id = match params.find("room_id") {
            Some(room_id) => RoomId::try_from(room_id).map_api_err(|_| {
                ApiError::not_found(Some(&format!("No room found with ID {}", room_id)))
            }),
            None => {
                Err(ApiError::missing_param("room_id"))
            }
        }?;
        request.extensions.insert::<RoomIdParam>(room_id);
        Ok(())
    }
}

/// Extracts a `UserId` from the URL path parameter `user_id`.
pub struct UserIdParam;

impl Key for UserIdParam {
    type Value = UserId;
}

impl BeforeMiddleware for UserIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let user_id = match params.find("user_id") {
            Some(user_id) => match UserId::try_from(user_id) {
                Ok(uid) => uid,
                Err(err) => {
                    let error = ApiError::missing_param(err.description());

                    return Err(IronError::new(error.clone(), error));
                }
            },
            None => {
                let error = ApiError::missing_param("user_id");

                return Err(IronError::new(error.clone(), error));
            }
        };

        request.extensions.insert::<UserIdParam>(user_id);

        Ok(())
    }
}

/// Extracts the URL path parameter `type`.
pub struct DataTypeParam;

impl Key for DataTypeParam {
    type Value = String;
}

impl BeforeMiddleware for DataTypeParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let data_type = params.find("type")
            .ok_or(ApiError::missing_param("type"))
            .map_err(IronError::from)?;

        request.extensions.insert::<DataTypeParam>(data_type.to_string().clone());

        Ok(())
    }
}

/// Extracts `RoomAliasId` from the URL path paramater `room_alias`.
pub struct RoomAliasIdParam;

impl Key for RoomAliasIdParam {
    type Value = RoomAliasId;
}

impl BeforeMiddleware for RoomAliasIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let config = Config::from_request(request)?;

        let room_alias_id = match params.find("room_alias") {
            Some(room_alias) => {
                debug!("room_alias param: {}", room_alias);

                let room_alias_id = RoomAliasId::try_from(
                    &format!("#{}:{}", room_alias, config.domain)
                ).map_api_err(|_| {
                    ApiError::not_found(
                        Some(&format!("No room alias found with ID {}", room_alias))
                    )
                })?;

                room_alias_id
            }
            None => {
                let error = ApiError::missing_param("room_alias");

                return Err(IronError::new(error.clone(), error));
            }
        };

        request.extensions.insert::<RoomAliasIdParam>(room_alias_id);

        Ok(())
    }
}

/// Extracts `EventType` from the URL path paramater `event_type`.
pub struct EventTypeParam;

impl Key for EventTypeParam {
    type Value = EventType;
}

impl BeforeMiddleware for EventTypeParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let event_type = params.find("event_type")
            .ok_or(ApiError::missing_param("event_type"))
            .map_err(IronError::from)
            .map(EventType::from)?;

        request.extensions.insert::<EventTypeParam>(event_type);

        Ok(())
    }
}

/// Extracts the URL path paramater `transaction_id`.
pub struct TransactionIdParam;

impl Key for TransactionIdParam {
    type Value = String;
}

impl BeforeMiddleware for TransactionIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let transaction_id = params.find("transaction_id")
            .ok_or(ApiError::missing_param("transaction_id"))
            .map_err(IronError::from)?;

        request.extensions.insert::<TransactionIdParam>(transaction_id.to_string().clone());

        Ok(())
    }
}
