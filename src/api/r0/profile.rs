//! Endpoints for profile.

use bodyparser;
use iron::{Chain, Handler, IronResult, IronError, Plugin, Request, Response};
use iron::status::Status;

use config::Config;
use db::DB;
use error::ApiError;
use middleware::{AccessTokenAuth, JsonRequest, UserIdParam};
use modifier::SerializableResponse;
use profile::{Profile as DataProfile};
use user::User;

#[derive(Clone, Debug, Serialize)]
struct ProfileResponse {
    avatar_url: Option<String>,
    displayname: Option<String>,
}

/// The `/profile/:user_id` endpoint.
pub struct Profile;

impl Profile {
    /// Create a `Profile` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(Profile);

        chain.link_before(UserIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for Profile {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let connection = DB::from_request(request)?;

        let profile = DataProfile::find_by_user_id(&connection, user_id.clone())?;

        let response = match profile {
            Some(profile) => {
                ProfileResponse {
                    avatar_url: profile.avatar_url,
                    displayname: profile.displayname,
                }
            }
            None => {
                let error = ApiError::not_found(
                    Some(&format!("No displayname found with ID {}", user_id))
                );

                return Err(IronError::new(error.clone(), error));
            }
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[derive(Clone, Debug, Serialize)]
struct GetAvatarUrlResponse {
    avatar_url: String,
}

/// The `/profile/:user_id/avatar_url` endpoint.
pub struct GetAvatarUrl;

impl GetAvatarUrl {
    /// Create a `GetAvatarUrl` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(GetAvatarUrl);

        chain.link_before(UserIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for GetAvatarUrl {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let connection = DB::from_request(request)?;

        let profile = DataProfile::find_by_user_id(&connection, user_id.clone())?;

        let response = match profile {
            Some(profile) => {
                match profile.avatar_url {
                    Some(avatar_url) => {
                        GetAvatarUrlResponse {
                            avatar_url: avatar_url,
                        }
                    },
                    None => {
                        let error = ApiError::not_found(
                            Some(&format!("No displayname found with ID {}", user_id))
                        );

                        return Err(IronError::new(error.clone(), error));
                    }
                }
            }
            None => {
                let error = ApiError::not_found(
                    Some(&format!("No displayname found with ID {}", user_id))
                );

                return Err(IronError::new(error.clone(), error));
            }
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PutAvatarUrlResquest {
    avatar_url: Option<String>,
}

/// The `/profile/:user_id/avatar_url` endpoint.
pub struct PutAvatarUrl;

impl PutAvatarUrl {
    /// Create a `PutAvatarUrl` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(PutAvatarUrl);

        chain.link_before(JsonRequest);
        chain.link_before(UserIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for PutAvatarUrl {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let avatar_url_request = match request.get::<bodyparser::Struct<PutAvatarUrlResquest>>() {
            Ok(Some(avatar_url_request)) => avatar_url_request,
            Ok(None) | Err(_) => {
                let error = ApiError::bad_json(None);

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            let error = ApiError::limited_rate(
                Some(&format!("No user found with ID {}", user_id))
            );

            return Err(IronError::new(error.clone(), error));
        }

        DataProfile::update_avatar_url(&connection, &config.domain, user_id, avatar_url_request.avatar_url)?;

        Ok(Response::with(Status::Ok))
    }
}

#[derive(Clone, Debug, Serialize)]
struct GetDisplaynameResponse {
    displayname: String,
}

/// The `/profile/:user_id/displayname` endpoint.
pub struct GetDisplayname;

impl GetDisplayname {
    /// Create a `GetDisplayname` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(GetDisplayname);

        chain.link_before(UserIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for GetDisplayname {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        let connection = DB::from_request(request)?;

        let profile = DataProfile::find_by_user_id(&connection, user_id.clone())?;

        let response = match profile {
            Some(profile) => {
                match profile.displayname {
                    Some(displayname) => {
                        GetDisplaynameResponse {
                            displayname: displayname,
                        }
                    },
                    None => {
                        let error = ApiError::not_found(
                            Some(&format!("No displayname found with ID {}", user_id))
                        );

                        return Err(IronError::new(error.clone(), error));
                    }
                }
            }
            None => {
                let error = ApiError::not_found(
                    Some(&format!("No displayname found with ID {}", user_id))
                );

                return Err(IronError::new(error.clone(), error));
            }
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PutDisplaynameResquest {
    displayname: Option<String>,
}

/// The `/profile/:user_id/displayname` endpoint.
pub struct PutDisplayname;

impl PutDisplayname {
    /// Create a `PutDisplayname` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(PutDisplayname);

        chain.link_before(JsonRequest);
        chain.link_before(UserIdParam);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for PutDisplayname {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let displayname_request = match request.get::<bodyparser::Struct<PutDisplaynameResquest>>() {
            Ok(Some(displayname_request)) => displayname_request,
            Ok(None) | Err(_) => {
                let error = ApiError::bad_json(None);

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let user_id = request.extensions.get::<UserIdParam>()
            .expect("UserIdParam should ensure a UserId").clone();

        // Check if the given user_id corresponds to the authenticated user.
        if user_id != user.id {
            let error = ApiError::limited_rate(
                Some(&format!("No user found with ID {}", user_id))
            );

            return Err(IronError::new(error.clone(), error));
        }

        DataProfile::update_displayname(&connection, &config.domain, user_id, displayname_request.displayname)?;

        Ok(Response::with(Status::Ok))
    }
}


#[cfg(test)]
mod tests {
    use test::Test;
    use iron::status::Status;

    #[test]
    fn get_displayname_not_set() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carls:ruma.test";

        let get_displayname_path = format!(
            "/_matrix/client/r0/profile/{}/displayname?access_token={}",
            user_id,
            access_token
        );
        assert_eq!(
            test.get(&get_displayname_path).status,
            Status::NotFound
        );
    }

    #[test]
    fn get_avatar_url_not_set() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carls:ruma.test";

        let get_avatar_url = format!(
            "/_matrix/client/r0/profile/{}/avatar_url?access_token={}",
            user_id,
            access_token
        );
        assert_eq!(
            test.get(&get_avatar_url).status,
            Status::NotFound
        );
    }

    #[test]
    fn put_avatar_url() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carl:ruma.test";

        let put_avatar_url_path = format!(
            "/_matrix/client/r0/profile/{}/avatar_url?access_token={}",
            user_id,
            access_token
        );
        let response = test.put(&put_avatar_url_path, r#"{"avatar_url": "mxc://matrix.org/wefh34uihSDRGhw34"}"#);

        assert_eq!(response.status, Status::Ok);

        let get_avatar_url_path = format!(
            "/_matrix/client/r0/profile/{}/avatar_url?access_token={}",
            user_id,
            access_token,
        );
        let response = test.get(&get_avatar_url_path);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(
            response.json().find("avatar_url").unwrap().as_str().unwrap(),
            r#"mxc://matrix.org/wefh34uihSDRGhw34"#
        );
    }

    #[test]
    fn put_displayname() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carl:ruma.test";

        let put_displayname_path = format!(
            "/_matrix/client/r0/profile/{}/displayname?access_token={}",
            user_id,
            access_token
        );
        let response = test.put(&put_displayname_path, r#"{"displayname": "Bogus"}"#);

        assert_eq!(response.status, Status::Ok);

        let get_displayname_path = format!(
            "/_matrix/client/r0/profile/{}/displayname?access_token={}",
            user_id,
            access_token,
        );
        let response = test.get(&get_displayname_path);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(
            response.json().find("displayname").unwrap().as_str().unwrap(),
            r#"Bogus"#
        );
    }

    #[test]
    fn put_displayname_rated_limted() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carls:ruma.test";

        let put_displayname = format!(
            "/_matrix/client/r0/profile/{}/displayname?access_token={}",
            user_id,
            access_token,
        );
        let response = test.put(&put_displayname, r#"{"displayname": "Bogus"}"#);

        assert_eq!(response.status, Status::TooManyRequests);
    }

    #[test]
    fn put_avatar_url_rated_limted() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carls:ruma.test";

        let put_avatar_url = format!(
            "/_matrix/client/r0/profile/{}/avatar_url?access_token={}",
            user_id,
            access_token,
        );
        let response = test.put(&put_avatar_url, r#"{"avatar_url": "mxc://matrix.org/wefh34uihSDRGhw34"}"#);

        assert_eq!(response.status, Status::TooManyRequests);
    }

    #[test]
    fn get_profile_not_set() {
        let test = Test::new();
        let access_token = test.create_access_token();
        let user_id = "@carls:ruma.test";

        let get_profile = format!(
            "/_matrix/client/r0/profile/{}?access_token={}",
            user_id,
            access_token,
        );
        let response = test.get(&get_profile);

        assert_eq!(response.status, Status::NotFound);
    }
}
