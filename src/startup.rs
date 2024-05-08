use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgConnection;

use crate::routes;

pub async fn run(
    listener: TcpListener,
    db_connection: PgConnection,
) -> Result<Server, std::io::Error> {
    // wrap connection in smart pointer
    let db_connection = web::Data::new(db_connection);

    let server = HttpServer::new(move || {
        App::new()
            .route("/ping", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .app_data(db_connection.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
