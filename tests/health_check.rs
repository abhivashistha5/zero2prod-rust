use sqlx::Executor;
use std::net::TcpListener;

use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod_rust::configuration::{self, DatabaseSettings};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

#[allow(clippy::let_underscore_future)]
async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let mut config = configuration::get_configuration().expect("Failed to load configuration");

    // create a temp database name
    config.database.database_name = format!("test_{}", Uuid::new_v4());

    let port = listener.local_addr().unwrap().port();

    let db_pool = configure_database(&config.database).await;
    let server = zero2prod_rust::startup::run(listener, db_pool.clone())
        .await
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(&config.connection_string_without_db_name())
        .await
        .expect("Failed to connect to database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations");

    connection_pool
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
