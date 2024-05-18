use sqlx::PgPool;

use crate::helpers::spawn_app;

#[sqlx::test]
async fn confirmation_without_token_are_rejected_with_bad_request(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
}
