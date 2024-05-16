use zero2prod_rust::{
    configuration,
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Setup telemetry
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = configuration::get_configuration().expect("Failed to load config");

    Application::build(&config, get_connection_pool(&config.database))
        .await
        .expect("Failed to build application")
        .run_until_stopped()
        .await
}
