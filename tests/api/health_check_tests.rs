use sqlx::PgPool;

use crate::helpers::spawn_app;

#[sqlx::test]
async fn health_check_works(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/ping", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(5), response.content_length());
}
