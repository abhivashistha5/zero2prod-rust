use sqlx::PgPool;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[sqlx::test]
async fn confirmation_without_token_are_rejected_with_bad_request(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn link_sent_in_mail_returns_200_when_called(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[sqlx::test]
async fn confirmation_link_confirms_a_subscriber(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = app.get_confirmation_links(email_request);

    let _ = reqwest::get(confirmation_links.html).await.unwrap();

    let subscriber = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(subscriber.email, "bruce@wayne.com");
    assert_eq!(subscriber.name, "Bruce Wayne");
    assert_eq!(subscriber.status, "CONFIRMED");
}
