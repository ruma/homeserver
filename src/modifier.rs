//! Iron modifiers.
use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;

use iron::Response;
use iron::headers::{ContentEncoding, ContentType, Encoding};
use iron::modifier::Modifier;

use serde::Serialize;
use serde_json::to_string;


/// Set the response's Content-Type header to "application/json" and set its body to the `T`
/// serialized to JSON. This modifier will also gzip compress the body of the response.
#[derive(Clone, Debug)]
pub struct SerializableResponse<T: Serialize>(pub T);

impl<T> Modifier<Response> for SerializableResponse<T> where T: Serialize {
    fn modify(self, response: &mut Response) {
        response.headers.set(ContentType::json());
        response.headers.set(ContentEncoding(vec![Encoding::Gzip]));
        let body = compress(to_string(&self.0).expect("could not serialize response data"));
        response.body = Some(Box::new(body));
    }
}

fn compress(s: String) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::Best);
    let bytes = s.into_bytes();
    let _ = encoder.write_all(&bytes);
    encoder.finish().unwrap()
}
