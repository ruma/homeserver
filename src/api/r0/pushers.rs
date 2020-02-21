//! Endpoints for pushers.
use bodyparser;
use iron::status::Status;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use serde_json::{from_value, Value};

use crate::db::DB;
use crate::error::{ApiError, MapApiError};
use crate::middleware::{AccessTokenAuth, JsonRequest, MiddlewareChain};
use crate::models::pusher::{Pusher, PusherOptions};
use crate::models::user::User;
use crate::modifier::{EmptyResponse, SerializableResponse};

/// The GET `/pushers` endpoint.
#[derive(Clone, Copy, Debug)]
pub struct GetPushers;

/// The body of the response for this API.
#[derive(Clone, Debug, Serialize)]
struct GetPushersResponse {
    /// A list of the `PusherOptions` for the user.
    pushers: Vec<PusherOptions>,
}

middleware_chain!(GetPushers, [AccessTokenAuth]);

impl Handler for GetPushers {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let user = request
            .extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        let connection = DB::from_request(request)?;

        let pushers = Pusher::find_by_uid(&connection, &user.id)?;

        let response = GetPushersResponse {
            pushers: pushers.into_iter().map(PusherOptions::from).collect(),
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

/// The POST `/pushers/set` endpoint.
#[derive(Clone, Copy, Debug)]
pub struct SetPushers;

middleware_chain!(SetPushers, [JsonRequest, AccessTokenAuth]);

impl Handler for SetPushers {
    fn handle(&self, request: &mut Request<'_, '_>) -> IronResult<Response> {
        let user = request
            .extensions
            .get::<User>()
            .expect("AccessTokenAuth should ensure a user")
            .clone();

        let value: Value = match request.get::<bodyparser::Struct<Value>>() {
            Ok(Some(request)) => request,
            Ok(None) | Err(_) => Err(IronError::from(ApiError::bad_json(None)))?,
        };

        let connection = DB::from_request(request)?;

        let borrowed_value = value.clone();
        let kind = borrowed_value
            .get("kind")
            .ok_or_else(|| ApiError::missing_param("kind"))?;

        if *kind == Value::Null {
            let app_id = value
                .get("app_id")
                .ok_or_else(|| ApiError::missing_param("app_id"))?;
            let app_id = app_id.as_str().ok_or_else(|| {
                ApiError::bad_json("The app_id parameter should be a string".to_string())
            })?;
            Pusher::delete(&connection, &user.id, app_id)?;
        } else {
            let pusher_options = from_value(value).map_api_err(ApiError::from)?;
            Pusher::upsert(&connection, &user.id, &pusher_options)?;
        }

        Ok(Response::with(EmptyResponse(Status::Ok)))
    }
}

#[cfg(test)]
mod tests {
    use crate::models::pusher::PusherData;
    use crate::models::pusher::PusherOptions;
    use crate::test::Test;
    use iron::status::Status;
    use serde_json::from_value;

    #[test]
    fn add_pusher() {
        let test = Test::new();
        let carl = test.create_user();
        let options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData {
                url: Some("test.de".to_string()),
            },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: false,
        };

        let response = test.set_pusher(&carl.token, options.clone());
        assert_eq!(response.status, Status::Ok);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", carl.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let mut pushers = response
            .json()
            .get("pushers")
            .unwrap()
            .as_array()
            .unwrap()
            .iter();
        assert_eq!(pushers.len(), 1);
        let pusher = pushers.next().unwrap().clone();
        let pusher: PusherOptions = from_value(pusher).unwrap();
        assert_eq!(pusher, options);
    }

    #[test]
    fn pusher_url_should_not_be_null_when_kind_is_http() {
        let test = Test::new();
        let carl = test.create_user();
        let options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData { url: None },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: false,
        };

        let response = test.set_pusher(&carl.token, options);
        assert_eq!(response.status, Status::UnprocessableEntity);
    }

    #[test]
    fn delete_pusher() {
        let test = Test::new();
        let carl = test.create_user();
        let options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData {
                url: Some("test.de".to_string()),
            },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: false,
        };

        let response = test.set_pusher(&carl.token, options);
        assert_eq!(response.status, Status::Ok);

        let post_pusher = format!(
            "/_matrix/client/r0/pushers/set?access_token={}",
            &carl.token,
        );
        let response = test.post(&post_pusher, r#"{"kind":null, "app_id":"device"}"#);
        assert_eq!(response.status, Status::Ok);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", carl.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        assert_eq!(json.get("pushers").unwrap().as_array().unwrap().len(), 0);
    }

    #[test]
    fn update_pusher() {
        let test = Test::new();
        let carl = test.create_user();
        let mut options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData {
                url: Some("test.de".to_string()),
            },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: false,
        };

        let response = test.set_pusher(&carl.token, options.clone());
        assert_eq!(response.status, Status::Ok);

        options.lang = "de".to_string();
        let response = test.set_pusher(&carl.token, options);
        assert_eq!(response.status, Status::Ok);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", carl.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        let mut pushers = json.get("pushers").unwrap().as_array().unwrap().iter();
        assert_eq!(
            pushers
                .next()
                .unwrap()
                .get("lang")
                .unwrap()
                .as_str()
                .unwrap(),
            "de"
        );
    }

    #[test]
    fn delete_different_users_pusher() {
        let test = Test::new();
        let carl = test.create_user();
        let alice = test.create_user();

        let options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData {
                url: Some("test.de".to_string()),
            },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: false,
        };

        let response = test.set_pusher(&carl.token, options.clone());
        assert_eq!(response.status, Status::Ok);

        let response = test.set_pusher(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", carl.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        assert_eq!(json.get("pushers").unwrap().as_array().unwrap().len(), 0);
    }

    #[test]
    fn delete_not_different_users_pusher() {
        let test = Test::new();
        let carl = test.create_user();
        let alice = test.create_user();

        let options = PusherOptions {
            lang: "en".to_string(),
            kind: "http".to_string(),
            data: PusherData {
                url: Some("test.de".to_string()),
            },
            device_display_name: "device".to_string(),
            app_id: "device".to_string(),
            profile_tag: Some("device".to_string()),
            pushkey: "device".to_string(),
            app_display_name: "device".to_string(),
            append: true,
        };

        let response = test.set_pusher(&carl.token, options.clone());
        assert_eq!(response.status, Status::Ok);

        let response = test.set_pusher(&alice.token, options);
        assert_eq!(response.status, Status::Ok);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", alice.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        assert_eq!(json.get("pushers").unwrap().as_array().unwrap().len(), 1);

        let get_pusher = format!("/_matrix/client/r0/pushers?access_token={}", carl.token,);
        let response = test.get(&get_pusher);
        assert_eq!(response.status, Status::Ok);
        let json = response.json();
        assert_eq!(json.get("pushers").unwrap().as_array().unwrap().len(), 1);
    }
}
