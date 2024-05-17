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

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "bruce@wayne.com");
    assert_eq!(saved.name, "Bruce Wayne");
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

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
