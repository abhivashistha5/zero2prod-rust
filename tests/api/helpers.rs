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
