use sqlx::PgPool;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[sqlx::test]
async fn subscribe_returns_200_valid_form_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(reqwest::StatusCode::OK, response.status());
}

#[sqlx::test]
async fn subscribe_saves_data_in_db(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "bruce@wayne.com");
    assert_eq!(saved.name, "Bruce Wayne");
    assert_eq!(saved.status, "PENDING_CONFIRMATION");
}

#[sqlx::test]
async fn subscribe_returns_400_invalid_request(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let inputs = vec![
        ("name=Bruce%20Wayne", "Missing email"),
        ("email=bruce%40wayne.com", "Missing name"),
        ("", "Missing name and email"),
    ];

    for (body, error_message) in inputs {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            reqwest::StatusCode::BAD_REQUEST,
            response.status(),
            "Api did not failed with bad request error: {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn subscribe_returns_400_on_empty_name(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let inputs = vec![("name=&email=bruce%40wayne.com", "Missing name")];

    for (body, error_message) in inputs {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            reqwest::StatusCode::BAD_REQUEST,
            response.status(),
            "Api did not failed with bad request error: {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_mail_for_valid_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_mail_with_a_link(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    tracing::trace!("Response: {:?}", response);
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let email_request = &app.email_server.received_requests().await.unwrap();

    assert_eq!(email_request.len(), 1);

    let confirmation_links = app.get_confirmation_links(&email_request[0]);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[sqlx::test]
async fn subscribe_fails_if_fatal_database_error(db_pool: PgPool) {
    // Arrange
    let app = spawn_app(db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
}
