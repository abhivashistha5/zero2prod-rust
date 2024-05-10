use actix_web::{
    web::{self, Form},
    HttpResponse, Responder,
};
use sqlx::{types::chrono::Utc, PgPool};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(form: Form<FormData>, db_pool: web::Data<PgPool>) -> impl Responder {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Saving new subscriber",
        %request_id,
        subs_name = %form.name,
        email = %form.email
    );
    let _request_span_guard = request_span.enter();

    let insert_result = sqlx::query!(
        r#"
    INSERT INTO subscriptions(id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    .await;

    match insert_result {
        Ok(_) => {
            tracing::info!("Subscriber save success");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
