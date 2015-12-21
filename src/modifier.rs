use iron::Response;
use iron::modifier::Modifier;
use serde::Serialize;
use serde_json::to_string;

#[derive(Clone, Debug)]
pub struct SerializableResponse<T: Serialize>(pub T);

impl<T> Modifier<Response> for SerializableResponse<T> where T: Serialize {
    fn modify(self, response: &mut Response) {
        response.body = Some(Box::new(to_string(&self.0).expect("could not serialize response data")));
    }
}
