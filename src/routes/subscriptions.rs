use actix_web::{
    web::{self, Form},
    HttpResponse, Responder,
};
use sqlx::{types::chrono::Utc, PgConnection};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(
    form: Form<FormData>,
    db_connection: web::Data<PgConnection>,
) -> impl Responder {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions(id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_connection.get_ref())
    .await;
    HttpResponse::Ok().finish()
}
