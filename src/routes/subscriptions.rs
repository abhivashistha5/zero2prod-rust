use actix_web::{
    web::{self, Form},
    HttpResponse,
};
use sqlx::{types::chrono::Utc, PgPool};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
pub async fn subscribe(form: Form<FormData>, db_pool: web::Data<PgPool>) -> HttpResponse {
    let subs_name = match SubscriberName::parse(form.0.name) {
        Ok(name) => name,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    let subs_email = match SubscriberEmail::parse(form.0.email) {
        Ok(email) => email,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    let new_subscriber = NewSubscriber {
        email: subs_email,
        name: subs_name,
    };

    match insert_subscriber(&new_subscriber, db_pool.get_ref()).await {
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
    skip(new_subscriber, db_pool)
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    db_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions(id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
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
