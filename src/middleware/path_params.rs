use std::convert::TryFrom;
use std::convert::From;
use std::error::Error;

use iron::{BeforeMiddleware, IronResult, Request};
use iron::typemap::Key;
use router::Router;
use ruma_events::EventType;
use ruma_identifiers::{
    UserId,
    RoomAliasId,
    RoomId,
    RoomIdOrAliasId
};

use config::Config;
use error::{ApiError, MapApiError};
use url::percent_encoding::percent_decode;

/// Extracts a `RoomId` from the URL path parameter `room_id`.
pub struct RoomIdParam;

impl Key for RoomIdParam {
    type Value = RoomId;
}

impl BeforeMiddleware for RoomIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();
        let room_id = match params.find("room_id") {
            Some(room_id) => {
                let decoded_room_id = percent_decode(room_id.as_bytes())
                    .decode_utf8()
                    .map_err(|err| {
                        ApiError::invalid_param("room_id", err.description())
                    })?;

                RoomId::try_from(&decoded_room_id).map_api_err(|err| {
                    ApiError::invalid_param("room_id", err.description())
                })
            },
            None => Err(ApiError::missing_param("room_id"))
        }?;
        request.extensions.insert::<RoomIdParam>(room_id);
        Ok(())
    }
}

/// Extracts a `RoomIdOrAlias` from the URL path parameter `room_id_or_alias`.
pub struct RoomIdOrAliasParam;

impl Key for RoomIdOrAliasParam {
    type Value = RoomIdOrAliasId;
}

impl BeforeMiddleware for RoomIdOrAliasParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>().expect("Params object is missing").clone();
        let room_id_or_alias = match params.find("room_id_or_alias") {
            Some(room_id_or_alias) => {
                let decoded_room_id_or_alias = percent_decode(room_id_or_alias.as_bytes())
                    .decode_utf8()
                    .map_err(|err| {
                        ApiError::invalid_param("room_id_or_alias", err.description())
                    })?;

                RoomIdOrAliasId::try_from(&decoded_room_id_or_alias).map_api_err(|err| {
                    ApiError::invalid_param("room_id_or_alias", err.description())
                })
            },
            None => Err(ApiError::missing_param("room_id_or_alias"))
        }?;
        request.extensions.insert::<RoomIdOrAliasParam>(room_id_or_alias);
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
            Some(user_id) => {
                let decoded_user_id = percent_decode(user_id.as_bytes())
                    .decode_utf8()
                    .map_err(|err| {
                        ApiError::invalid_param("user_id", err.description())
                    })?;

                UserId::try_from(&decoded_user_id).map_api_err(|err| {
                    ApiError::invalid_param("user_id", err.description())
                })
            },
            None => Err(ApiError::missing_param("user_id"))
        }?;

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
            .ok_or_else(||ApiError::missing_param("type"))?;

        request.extensions.insert::<DataTypeParam>(data_type.to_string().clone());

        Ok(())
    }
}


/// Extracts the URL path parameter `filter_id`.
pub struct FilterIdParam;

impl Key for FilterIdParam {
    type Value = i64;
}

impl BeforeMiddleware for FilterIdParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let filter_id = params.find("filter_id")
            .ok_or_else(||ApiError::missing_param("filter_id"))?;
        let filter_id: i64 = filter_id.parse()
            .map_err(|_| ApiError::invalid_param("filter_id", "Parsing failed"))?;

        request.extensions.insert::<FilterIdParam>(filter_id);

        Ok(())
    }
}

/// Extracts `RoomAliasId` from the URL path parameter `room_alias`.
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

                RoomAliasId::try_from(
                    &format!("#{}:{}", room_alias, config.domain)
                ).map_api_err(|err| {
                    ApiError::invalid_param("room_alias", err.description())
                })?
            }
            None => Err(ApiError::missing_param("room_alias"))?,
        };

        request.extensions.insert::<RoomAliasIdParam>(room_alias_id);

        Ok(())
    }
}

/// Extracts `EventType` from the URL path parameter `event_type`.
pub struct EventTypeParam;

impl Key for EventTypeParam {
    type Value = EventType;
}

impl BeforeMiddleware for EventTypeParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let event_type = params.find("event_type")
            .ok_or_else(||ApiError::missing_param("event_type"))
            .map(EventType::from)?;

        request.extensions.insert::<EventTypeParam>(event_type);

        Ok(())
    }
}

/// Extracts the URL path paramater `tag`.
pub struct TagParam;

impl Key for TagParam {
    type Value = String;
}

impl BeforeMiddleware for TagParam {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let params = request.extensions.get::<Router>()
            .expect("Params object is missing").clone();

        let tag = params.find("tag")
            .ok_or_else(||ApiError::missing_param("tag"))?;

        request.extensions.insert::<TagParam>(tag.to_string().clone());

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
            .ok_or_else(||ApiError::missing_param("transaction_id"))?;

        request.extensions.insert::<TransactionIdParam>(transaction_id.to_string().clone());

        Ok(())
    }
}
