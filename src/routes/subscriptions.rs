use actix_web::{
    web::{self, Form},
    HttpResponse,
};
use sqlx::{types::chrono::Utc, PgPool};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
}

#[tracing::instrument(
    name = "Saving a new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subs_name = %form.name,
        email = %form.email
    )
)]
pub async fn subscribe(
    form: Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(sub) => sub,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    let confirmation_link = generate_confirmation_link(&base_url.0);

    if insert_subscriber(&new_subscriber, db_pool.get_ref())
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_link(
        email_client.as_ref(),
        &new_subscriber.email,
        confirmation_link,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    tracing::info!("Subscriber save success");
    HttpResponse::Ok().finish()
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
    INSERT INTO subscriptions(id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, $5)
    "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        "PENDING_CONFIRMATION",
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation link"
    skip(email_client, email, confirmation_link)
)]
pub async fn send_confirmation_link(
    email_client: &EmailClient,
    email: &SubscriberEmail,
    confirmation_link: String,
) -> Result<(), reqwest::Error> {
    email_client
        .send_email(
            email,
            "Welcome!",
            &format!(r#"Welcome to our newsletter!<br/> Click <a href="{}">here</a> to confirm your subscription"#, confirmation_link),
            &format!(r#"Welcome to our newsletter!\nVisit {} to confirm your subscription"#, confirmation_link),
        )
        .await.map_err(|e| {
        tracing::error!("Failed to send confirmation mail: {}", e);
        e
        })?;

    Ok(())
}

#[tracing::instrument(name = "Generating confirmation link")]
pub fn generate_confirmation_link(base_url: &str) -> String {
    format!(
        "{}/subscriptions/confirm?subscription_token=mytoken",
        base_url
    )
}
