use sqlx::PgPool;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=bruce%20wayne&email=bruce%40wayne.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[sqlx::test]
async fn newsletter_should_not_publish_to_pending_subscribers(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter as plain text",
            "html": "<p>Newsletter as html</p>",
        }
    });

    let response = app.publish_newsletter(newsletter_request_body).await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_should_publish_to_confirmed_subscribers(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    create_confirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter as plain text",
            "html": "<p>Newsletter as html</p>",
        }
    });

    let response = app.publish_newsletter(newsletter_request_body).await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_returns_400_for_invalid_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let test_requests: Vec<(serde_json::value::Value, &str)> = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter as plain text",
                    "html": "<p>Newsletter as html</p>",
                }
            }),
            "Missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
            }),
            "Missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "content": {
                    "html": "<p>Newsletter as html</p>",
                }
            }),
            "Missing text content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter Title",
                "content": {
                    "text": "Newsletter as plain text",
                }
            }),
            "Missing html content",
        ),
    ];

    for (invalid_body, error_message) in test_requests {
        let response = app.publish_newsletter(invalid_body).await;

        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "Api not failed with 400 BAD_REQUEST for payload: {}",
            error_message
        );
    }
}
