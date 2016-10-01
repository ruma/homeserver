use bodyparser;
use diesel::SaveChangesDsl;
use diesel::result::Error as DieselError;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;

use crypto::hash_password;
use db::DB;
use error::ApiError;
use middleware::{
    AccessTokenAuth,
    JsonRequest,
    DataTypeParam,
    UserIdParam,
};
use user::User;
use access_token::AccessToken;
use account_data::{AccountData, NewAccountData};

/// The /account/password endpoint.
#[derive(Debug)]
pub struct AccountPassword;

#[derive(Clone, Debug, Deserialize)]
struct AccountPasswordRequest {
    pub new_password: String,
}

impl AccountPassword {
    /// Create an `AccountPassword` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(AccountPassword);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for AccountPassword {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let account_password_request = match request
            .get::<bodyparser::Struct<AccountPasswordRequest>>()
        {
            Ok(Some(account_password_request)) => account_password_request,
            Ok(None) | Err(_) => {
                let error = ApiError::not_json(None);

                return Err(IronError::new(error.clone(), error));
            }
        };

        let mut user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        user.password_hash = hash_password(&account_password_request.new_password)?;

        let connection = DB::from_request(request)?;

        if let Err(_) = user.save_changes::<User>(&*connection) {
            let error = ApiError::unauthorized(None);

            return Err(IronError::new(error.clone(), error));
        }

        Ok(Response::with(Status::Ok))
    }
}


/// The /account/deactivate endpoint.
#[derive(Debug)]
pub struct DeactivateAccount;

impl DeactivateAccount {
    /// Create a `DeactivateAccount` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(DeactivateAccount);

        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for DeactivateAccount {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let connection = DB::from_request(request)?;

        {
            let token = request.extensions.get_mut::<AccessToken>()
                .expect("AccessTokenAuth should ensure an access token");

            if let Err(error) = token.revoke(&connection) {
                return Err(IronError::new(error.clone(), error));
            };
        }

        let user = request.extensions.get_mut::<User>()
            .expect("AccessTokenAuth should ensure a user");

        if let Err(error) = user.deactivate(&connection) {
            return Err(IronError::new(error.clone(), error));
        };

        // Delete all the account data associated with the user.
        if let Err(error) = AccountData::delete_by_uid(&connection, user.id.clone()) {
            return Err(IronError::new(error.clone(), error));
        };

        Ok(Response::with(Status::Ok))
    }
}


/// The /user/:user_id/account_data/:type endpoint.
#[derive(Debug)]
pub struct PutAccountData;

impl PutAccountData {
    /// Create an `PutAccountData` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(PutAccountData);

        chain.link_before(JsonRequest);
        chain.link_before(UserIdParam);
        chain.link_before(DataTypeParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for PutAccountData {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            let error = ApiError::not_found(
                Some(&format!("No user found with ID {}", user_id))
            );

            return Err(IronError::new(error.clone(), error));
        }

        let data_type = request.extensions.get::<DataTypeParam>()
            .expect("DataTypeParam should ensure a data type").clone();

        let content = match request.get::<bodyparser::Json>() {
            Ok(Some(content)) => content.to_string().clone(),
            Ok(None) | Err(_) => {
                let error = ApiError::bad_json(None);

                return Err(IronError::new(error.clone(), error));
            }
        };

        let new_data = NewAccountData {
            user_id: user.id,
            data_type: String::from(data_type),
            content: content,
        };

        let connection = DB::from_request(request)?;

        // Insert or update an existing AccountData entry.
        match AccountData::find_by_uid_and_type(
            &connection,
            &new_data.user_id,
            &new_data.data_type
        ) {
            Ok(mut saved_data) => {
                if let Err(err) = saved_data.update(&connection, new_data.content) {
                    return Err(IronError::new(err.clone(), err));
                }
            }
            Err(err) => {
                match err {
                    DieselError::NotFound => {
                        if let Err(err) = AccountData::create(&connection, &new_data) {
                            let error = ApiError::from(err);

                            return Err(IronError::new(error.clone(), error));
                        }
                    }
                    _ => {
                        let error = ApiError::from(err);

                        return Err(IronError::new(error.clone(), error));
                    }
                }
            }
        }

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
        let access_token = test.create_access_token();

        assert!(
            test.post(
                &format!("/_matrix/client/r0/account/password?access_token={}", access_token),
                r#"{"new_password": "hidden"}"#
            ).status.is_success()
        );

        assert!(
            test.post(
                "/_matrix/client/r0/login",
                r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "hidden"}}"#,
            ).status.is_success()
        )
    }

    #[test]
    fn deactivate_account() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let login = r#"{"auth": {"type": "m.login.password", "user": "carl", "password": "secret"}}"#;
        let deactivate = format!("/_matrix/client/r0/account/deactivate?access_token={}", access_token);

        assert!(
            test.post("/_matrix/client/r0/login", login).status.is_success()
        );

        assert!(
            test.post(&deactivate, r#"{}"#).status.is_success()
        );

        assert_eq!(
            test.post("/_matrix/client/r0/login", login).status,
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
        let access_token = test.create_access_token();
        let user_id = "@carl:ruma.test";

        let content = r#"{"email": "user@email.com", "phone": "123456789"}"#;
        let data_type = "org.matrix.personal.config";
        let account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user_id, data_type, access_token
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
        let access_token = test.create_access_token();
        let mut user_id = "carl.ruma.test";

        let content = r#"{"email": "user@email.com", "phone": "123456789"}"#;
        let data_type = "org.matrix.personal.config";
        let mut account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user_id, data_type, access_token
        );

        // Invalid UserId.
        assert_eq!(
            test.put(&account_data_path, &content).status,
            Status::BadRequest
        );

        // Non-existent user.
        user_id = "@mark:ruma.test";
        account_data_path = format!(
            "/_matrix/client/r0/user/{}/account_data/{}?access_token={}",
            user_id, data_type, access_token
        );

        assert_eq!(
            test.put(&account_data_path, &content).status,
            Status::NotFound
        );
    }
}
