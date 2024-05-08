use std::net::TcpListener;

use sqlx::{Connection, PgConnection};
use zero2prod_rust::configuration;

#[allow(clippy::let_underscore_future)]
async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod_rust::startup::run(listener)
        .await
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/ping", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(5), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    let address = spawn_app().await;
    let config = configuration::get_configuration().expect("Failed to load configuration");
    let connection_string = config.database.connection_string();
    let mut db = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to db");

    let client = reqwest::Client::new();
    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    let response = client
        .post(format!("{}/subscriptions", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(reqwest::StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut db)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "bruce@wayne.com");
    assert_eq!(saved.name, "Bruce Wayne");
}

#[tokio::test]
async fn subscribe_returns_400_invalid_request() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let inputs = vec![
        ("name=Bruce%20Wayne", "Missing email"),
        ("email=bruce%40wayne.com", "Missing name"),
        ("", "Missing name and email"),
    ];

    for (body, error_message) in inputs {
        let response = client
            .post(format!("{}/subscriptions", address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            reqwest::StatusCode::BAD_REQUEST,
            response.status(),
            "Api did not failed with bad request error: {}",
            error_message
        );
        // assert_eq!(error_message, response.text().await.unwrap().as_str());
    }
}
