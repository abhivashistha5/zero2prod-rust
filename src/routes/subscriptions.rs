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

#[tracing::instrument(
    name = "Saving a new subscriber",
    skip(form, db_pool),
    fields(
        subs_name = %form.name,
        email = %form.email
    )
)]
pub async fn subscribe(form: Form<FormData>, db_pool: web::Data<PgPool>) -> impl Responder {
    match insert_subscriber(&form, db_pool.get_ref()).await {
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

#[tracing::instrument(
    name = "Saving subscriber in db"
    skip(form, db_pool)
)]
pub async fn insert_subscriber(form: &FormData, db_pool: &PgPool) -> Result<(), sqlx::Error> {
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
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {}", e);
        e
    })?;

    Ok(())
}
