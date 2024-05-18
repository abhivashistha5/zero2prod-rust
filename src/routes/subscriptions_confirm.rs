use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct SubConfirmationParam {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm pending subscriber", skip(param))]
pub async fn confirm(param: web::Query<SubConfirmationParam>) -> HttpResponse {
    tracing::trace!("Parameters recieved: {:?}", param.subscription_token);
    HttpResponse::Ok().finish()
}
