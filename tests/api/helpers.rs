use once_cell::sync::Lazy;
use sqlx::PgPool;
use wiremock::MockServer;
use zero2prod_rust::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_links = |s: &str| {
            let links: Vec<linkify::Link> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);
            let raw_confirmation_link = links[0].as_str().to_owned();
            let mut confirmation_link: reqwest::Url =
                reqwest::Url::parse(&raw_confirmation_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "localhost");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html_link = get_links(body["HtmlBody"].as_str().unwrap());
        let plain_text_link = get_links(body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html: html_link,
            plain_text: plain_text_link,
        }
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    // setup telemetry
    // tracing can be initialized only once and running in
    // spawn app leads to runtime error
    //
    // That is why wrapping it up in once_cell
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

#[allow(clippy::let_underscore_future)]
pub async fn spawn_app(db_pool: PgPool) -> TestApp {
    // This will be called only once
    Lazy::force(&TRACING);

    let mut config = get_configuration().expect("Configuration load failed");
    let email_server = MockServer::start().await;

    // override config for test
    config.application.port = 0; // for selecting random port
    config.email.base_url = email_server.uri();

    let app: Application = Application::build(config, db_pool.clone())
        .await
        .expect("Failed to start server");
    let address = format!("http://127.0.0.1:{}", app.port);
    let port = app.port;

    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address,
        port,
        db_pool,
        email_server,
    }
}
