#[tokio::test]
async fn health_check_works() {
    spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get("http://127.0.0.1:8000/ping")
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(5), response.content_length());
}

#[allow(clippy::let_underscore_future)]
async fn spawn_app() {
    let server = zero2prod_rust::run().await.expect("Failed to bind address");
    let _ = tokio::spawn(server);
}
