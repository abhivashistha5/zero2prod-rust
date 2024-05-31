use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct PublishNLBody {
    title: String,
    content: PublishContent,
}

#[derive(serde::Deserialize)]
pub struct PublishContent {
    html: String,
    text: String,
}

#[tracing::instrument(name = "Publish Newsletter to subscriber", skip(_body))]
pub async fn publish_newsletter(_body: web::Json<PublishNLBody>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
