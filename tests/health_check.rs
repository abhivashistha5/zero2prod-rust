use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod_rust::configuration;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

#[allow(clippy::let_underscore_future)]
async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let config = configuration::get_configuration().expect("Failed to load configuration");

    let port = listener.local_addr().unwrap().port();
    let connection_string = config.database.connection_string();
    let db_pool = PgPool::connect(&connection_string)
        .await
        .expect("Failed to connect to db");
    let server = zero2prod_rust::startup::run(listener, db_pool.clone())
        .await
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool,
    }
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/ping", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(5), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();
    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    let response = client
        .post(format!("{}/subscriptions", app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(reqwest::StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "bruce@wayne.com");
    assert_eq!(saved.name, "Bruce Wayne");
}

#[tokio::test]
async fn subscribe_returns_400_invalid_request() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let inputs = vec![
        ("name=Bruce%20Wayne", "Missing email"),
        ("email=bruce%40wayne.com", "Missing name"),
        ("", "Missing name and email"),
    ];

    for (body, error_message) in inputs {
        let response = client
            .post(format!("{}/subscriptions", app.address))
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
    }
}
