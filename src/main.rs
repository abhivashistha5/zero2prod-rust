use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
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
    let address = format!("{}:{}", config.application.host, config.application.port);
    let db_connection_pool = PgPoolOptions::new().connect_lazy_with(config.database.with_db());
    let listener = TcpListener::bind(address).expect("Failed to bind to port");
    run(listener, db_connection_pool).await?.await
}
