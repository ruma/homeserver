extern crate hyper;
extern crate ruma;

mod support;

use std::io::Read;

use hyper::Ok;

use support::Test;

#[test]
fn it_works() {
    let test = Test::new();

    let mut response = test.post(
        "/_matrix/client/r0/register",
        r#"{"password": "secret"}"#,
    );

    let mut body = String::new();

    response.read_to_string(&mut body).unwrap();

    assert_eq!(body, "foo");
}
