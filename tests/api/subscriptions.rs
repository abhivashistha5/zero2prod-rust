use sqlx::PgPool;

use crate::helpers::spawn_app;

#[sqlx::test]
async fn subscribe_returns_200_valid_form_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let body = "name=Bruce%20Wayne&email=bruce%40wayne.com";

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
