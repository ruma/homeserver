use super::Test;

#[test]
fn minimum_input_parameters() {
    let test = Test::new();

    let response = test.post(
        "/_matrix/client/r0/register",
        r#"{"password": "secret"}"#,
    );

    assert!(response.json.find("access_token").is_some());
    assert_eq!(response.json.find("home_server").unwrap().as_string().unwrap(), "ruma.test");
    assert!(response.json.find("user_id").is_some());
}

#[test]
fn all_input_parameters() {
    let test = Test::new();

    let response = test.post(
        "/_matrix/client/r0/register",
        r#"{"bind_email": true, "kind": "user", "username":"carl", "password": "secret"}"#,
    );

    assert!(response.json.find("access_token").is_some());
    assert_eq!(response.json.find("home_server").unwrap().as_string().unwrap(), "ruma.test");
    assert_eq!(response.json.find("user_id").unwrap().as_string().unwrap(), "carl");
}

#[test]
fn guest_access_not_supported() {
    let test = Test::new();

    let response = test.post(
        "/_matrix/client/r0/register",
        r#"{"bind_email": true, "kind": "guest", "username":"carl", "password": "secret"}"#,
    );

    assert_eq!(
        response.json.find("errcode").unwrap().as_string().unwrap(),
        "M_GUEST_ACCESS_FORBIDDEN"
    );
}
