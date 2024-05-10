use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;
use zero2prod_rust::{
    configuration,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Setup telemetry
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = configuration::get_configuration().expect("Failed to load config");
    let address = format!("127.0.0.1:{}", config.application_port);
    let db_connection_pool = PgPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to database");
    let listener = TcpListener::bind(address).expect("Failed to bind to port");
    run(listener, db_connection_pool).await?.await
}
