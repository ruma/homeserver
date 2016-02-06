//! Iron modifiers.

use iron::Response;
use iron::headers::ContentType;
use iron::modifier::Modifier;
use serde::Serialize;
use serde_json::to_string;

/// Set the response's Content-Type header to "application/json" and set its body to the `T`
/// serialized to JSON.
#[derive(Clone, Debug)]
pub struct SerializableResponse<T: Serialize>(pub T);

impl<T> Modifier<Response> for SerializableResponse<T> where T: Serialize {
    fn modify(self, response: &mut Response) {
        response.headers.set(ContentType::json());
        response.body = Some(Box::new(to_string(&self.0).expect("could not serialize response data")));
    }
}
