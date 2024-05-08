use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod_rust::{configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = configuration::get_configuration().expect("Failed to load config");
    let address = format!("127.0.0.1:{}", config.application_port);
    let db_connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to database");
    let listener = TcpListener::bind(address).expect("Failed to bind to port");
    run(listener, db_connection_pool).await?.await
}
