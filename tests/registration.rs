use support::Test;

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
        "/_matrix/client/r0/register?kind=user",
        r#"{"bind_email": true, "username":"carl", "password": "secret"}"#,
    );

    assert!(response.json.find("access_token").is_some());
    assert_eq!(response.json.find("home_server").unwrap().as_string().unwrap(), "ruma.test");
    assert_eq!(response.json.find("user_id").unwrap().as_string().unwrap(), "carl");
}
