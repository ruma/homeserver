//! Endpoints for accounts.
use bodyparser;
use diesel::SaveChangesDsl;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;

use crypto::hash_password;
use db::DB;
use error::ApiError;
use middleware::{
    AccessTokenAuth,
    DataTypeParam,
    JsonRequest,
    MiddlewareChain,
    RoomIdParam,
    UserIdParam,
};
use models::access_token::AccessToken;
use models::account_data::{
    AccountData,
    NewAccountData,
    RoomAccountData,
    NewRoomAccountData,
};
use models::room_membership::RoomMembership;
use models::user::User;

/// The `/account/password` endpoint.
#[derive(Debug)]
pub struct AccountPassword;

#[derive(Clone, Debug, Deserialize)]
struct AccountPasswordRequest {
    pub new_password: String,
}

middleware_chain!(AccountPassword, [AccessTokenAuth]);

impl Handler for AccountPassword {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let account_password_request = match request
            .get::<bodyparser::Struct<AccountPasswordRequest>>()
        {
            Ok(Some(account_password_request)) => account_password_request,
            Ok(None) | Err(_) => Err(ApiError::not_json(None))?,
        };

        let mut user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        user.password_hash = hash_password(&account_password_request.new_password)?;

        let connection = DB::from_request(request)?;

        user.save_changes::<User>(&*connection)
            .map_err(|_| ApiError::unauthorized(None))?;

        Ok(Response::with(Status::Ok))
    }
}

/// The `/account/deactivate` endpoint.
#[derive(Debug)]
pub struct DeactivateAccount;

middleware_chain!(DeactivateAccount, [AccessTokenAuth]);

impl Handler for DeactivateAccount {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let connection = DB::from_request(request)?;

        {
            let token = request.extensions.get_mut::<AccessToken>()
                .expect("AccessTokenAuth should ensure an access token");

            token.revoke(&connection)?;
        }

        let user = request.extensions.get_mut::<User>()
            .expect("AccessTokenAuth should ensure a user");

        user.deactivate(&connection)?;

        // Delete all the account data associated with the user.
        AccountData::delete_by_uid(&connection, &user.id)?;
        RoomAccountData::delete_by_uid(&connection, &user.id)?;

        Ok(Response::with(Status::Ok))
    }
}

/// The `/user/:user_id/account_data/:type` endpoint.
#[derive(Debug)]
pub struct PutAccountData;

middleware_chain!(PutAccountData, [JsonRequest, UserIdParam, DataTypeParam, AccessTokenAuth]);

impl Handler for PutAccountData {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        if user_id != user.id {
            let error = ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string()
            );

            return Err(IronError::from(error));
        }

        let data_type = request.extensions.get::<DataTypeParam>()
            .expect("DataTypeParam should ensure a data type").clone();

        let content = match request.get::<bodyparser::Json>() {
            Ok(Some(content)) => content.to_string().clone(),
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };

        let new_data = NewAccountData {
            user_id: user.id,
            data_type: data_type.to_string(),
            content: content,
        };

        let connection = DB::from_request(request)?;

        AccountData::upsert(&connection, &new_data)?;

        Ok(Response::with(Status::Ok))
    }
}

/// The `/user/:user_id/rooms/:room_id/account_data/:type` endpoint.
#[derive(Debug)]
pub struct PutRoomAccountData;

middleware_chain!(PutRoomAccountData, [JsonRequest, UserIdParam, RoomIdParam, DataTypeParam, AccessTokenAuth]);

impl Handler for PutRoomAccountData {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        if user_id != user.id {
            let error = ApiError::unauthorized(
                "The given user_id does not correspond to the authenticated user".to_string()
            );

            return Err(IronError::from(error));
        }

        let room_id = request.extensions.get::<RoomIdParam>()
            .expect("RoomIdParam should ensure a RoomId").clone();

        let connection = DB::from_request(request)?;

        // Check if the user has joined the room.
        let entry = RoomMembership::find(&connection, &room_id, &user_id)?;

        if entry.is_none() {
            let error = ApiError::unauthorized(
                "No membership entry was found.".to_string()
            );

            return Err(IronError::from(error));
        }

        if entry.unwrap().membership != "join" {
            let error = ApiError::unauthorized("The room is not accesible.".to_string());

            return Err(IronError::from(error));
        }

        let data_type = request.extensions.get::<DataTypeParam>()
            .expect("DataTypeParam should ensure a data type").clone();

        let content = match request.get::<bodyparser::Json>() {
            Ok(Some(content)) => content.to_string().clone(),
            Ok(None) | Err(_) => Err(ApiError::bad_json(None))?,
        };

        let new_data = NewRoomAccountData {
            user_id: user.id,
            room_id: room_id,
            data_type: data_type.to_string(),
            content: content,
        };

        RoomAccountData::upsert(&connection, &new_data)?;

        Ok(Response::with(Status::Ok))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn change_password() {
        let test = Test::new();
        let user = test.create_user();

        assert!(
            test.post(
                &format!("/_matrix/client/r0/account/password?access_token={}", user.token),
                r#"{"new_password": "hidden"}"#
            ).status.is_success()
        );

        assert!(
            test.post(
                "/_matrix/client/r0/login",
                format!(r#"{{"type": "m.login.password", "user": "{}", "password": "hidden"}}"#, user.name).as_str(),
            ).status.is_success()
        )
    }

    #[test]
    fn deactivate_account() {
        let test = Test::new();
        let user = test.create_user();

        let login = format!(r#"{{"type": "m.login.password", "user": "{}", "password": "secret"}}"#, user.name);
        let deactivate = format!("/_matrix/client/r0/account/deactivate?access_token={}", user.token);

        assert!(
            test.post("/_matrix/client/r0/login", &login).status.is_success()
        );

        assert!(
            test.post(&deactivate, r#"{}"#).status.is_success()
        );

        assert_eq!(
            test.post("/_matrix/client/r0/login", &login).status,
            Status::Forbidden
        );

        assert_eq!(
            test.post(&deactivate, r#"{}"#).status,
            Status::Forbidden
        );
    }

    #[test]
    fn update_account_data() {
        let test = Test::new();
        let user = test.create_user();

        let content = r#"{"email": "user@email.com", "phone": "123456789"}"#;
        let data_type = "org.matrix.personal.config";
        let account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user.id, data_type, user.token
        );

        assert!(
            test.put(&account_data_path, &content).status.is_success()
        );

        let new_content = r#"{"email": "user@email.org", "phone": "123456789", "fax": "123456991"}"#;

        assert!(
            test.put(&account_data_path, &new_content).status.is_success()
        );
    }

    #[test]
    fn update_account_data_with_invalid_user_id() {
        let test = Test::new();
        let user = test.create_user();
        let mut user_id = "mark:ruma.test";

        let content = r#"{"email": "user@email.com", "phone": "123456789"}"#;
        let data_type = "org.matrix.personal.config";
        let mut account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user_id, data_type, user.token
        );

        let response = test.put(&account_data_path, &content);

        // Invalid UserId.
        assert_eq!(response.status, Status::BadRequest);
        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "IO_RUMA_INVALID_PARAM"
        );
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "Parameter 'user_id' is not valid: leading sigil is missing"
        );

        // Non-existent user.
        user_id = "@mark:ruma.test";
        account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user_id, data_type, user.token
        );

        assert_eq!(
            test.put(&account_data_path, &content).status,
            Status::Forbidden
        );
    }

    #[test]
    fn update_room_account_data() {
        let test = Test::new();
        let user = test.create_user();

        let room_id = test.create_public_room(&user.token);
        let content = r#"{"ui_color": "yellow"}"#;
        let data_type = "org.matrix.room.config";
        let path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/account_data/{}?access_token={}",
            user.id, room_id, data_type, user.token
        );

        assert_eq!(test.join_room(&user.token, &room_id).status, Status::Ok);

        assert_eq!(test.put(&path, &content).status, Status::Ok);

        let new_content = r#"{"ui_color": "yellow", "show_nicknames": "true"}"#;

        assert_eq!(test.put(&path, &new_content).status, Status::Ok);
    }

    #[test]
    fn update_room_account_data_with_invalid_user() {
        let test = Test::new();
        let user = test.create_user();

        let room_id = test.create_public_room(&user.token);
        let user_id = "@mark:ruma.test";
        let content = r#"{"ui_color": "yellow"}"#;
        let data_type = "org.matrix.room.config";

        let path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/account_data/{}?access_token={}",
            user_id, room_id, data_type, user.token
        );

        assert_eq!(test.join_room(&user.token, &room_id).status, Status::Ok);

        assert_eq!(test.put(&path, &content).status, Status::Forbidden);

        assert_eq!(
            test.put(&path, &content).json().get("error").unwrap().as_str().unwrap(),
            "The given user_id does not correspond to the authenticated user"
        );
    }

    #[test]
    fn update_room_account_data_with_invalid_room() {
        let test = Test::new();
        let carl = test.create_user();

        let room_id = "invalid_room_id";
        let content = r#"{"ui_color": "yellow"}"#;
        let data_type = "org.matrix.room.config";

        let path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/account_data/{}?access_token={}",
            carl.id, room_id, data_type, carl.token
        );

        let response = test.put(&path, &content);

        assert_eq!(response.status, Status::BadRequest);
        assert_eq!(
            response.json().get("errcode").unwrap().as_str().unwrap(),
            "IO_RUMA_INVALID_PARAM"
        );
        assert_eq!(
            response.json().get("error").unwrap().as_str().unwrap(),
            "Parameter 'room_id' is not valid: leading sigil is missing"
        );
    }

    #[test]
    fn update_room_account_data_without_room_access() {
        let test = Test::new();
        let carl = test.create_user();
        let mark = test.create_user();

        let room_id = test.create_private_room(&mark.token);
        let content = r#"{"ui_color": "yellow"}"#;
        let data_type = "org.matrix.room.config";

        let path = format!(
            "/_matrix/client/r0/user/{}/rooms/{}/account_data/{}?access_token={}",
            carl.id, room_id, data_type, carl.token
        );

        assert_eq!(test.put(&path, &content).status, Status::Forbidden);

        assert_eq!(
            test.put(&path, &content).json().get("error").unwrap().as_str().unwrap(),
            "No membership entry was found."
        );
    }
}

