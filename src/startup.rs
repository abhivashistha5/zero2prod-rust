use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes,
};
pub struct Application {
    pub port: u16,
    pub server: Server,
}
impl Application {
    pub async fn build(config: Settings, db_pool: PgPool) -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(address).expect("Failed to bind to port");

        let email_client = EmailClient::new(
            config.email.base_url.as_str(),
            config.email.sender(),
            config.email.authorization_token.clone(),
            config.email.timeout(),
        );

        Ok(Self {
            port: listener.local_addr().unwrap().port(),
            server: run(listener, db_pool, email_client, config.application.base_url).await?,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(db_config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(db_config.with_db())
}

pub struct ApplicationBaseUrl(pub String);

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // wrap connection in smart pointer
    let db_connection_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/ping", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .route("/subscriptions/confirm", web::get().to(routes::confirm))
            .route("/newsletter", web::post().to(routes::publish_newsletter))
            .app_data(db_connection_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
