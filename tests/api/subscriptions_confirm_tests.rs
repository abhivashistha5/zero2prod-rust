use reqwest::Url;
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
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_links = |s: &str| -> String {
        let links: Vec<linkify::Link> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();

        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let raw_confirmation_link = get_links(body["HtmlBody"].as_str().unwrap());
    let mut confirmation_link: Url = Url::parse(&raw_confirmation_link).unwrap();

    assert_eq!(confirmation_link.host_str().unwrap(), "localhost");

    confirmation_link.set_port(Some(app.port)).unwrap();

    let response = reqwest::get(confirmation_link).await.unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
