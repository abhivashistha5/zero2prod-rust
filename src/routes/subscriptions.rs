use std::ops::DerefMut;

use actix_web::{
    web::{self, Form},
    HttpResponse, ResponseError,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secrecy::ExposeSecret;
use sqlx::{types::chrono::Utc, PgPool, Postgres, Transaction};
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
pub struct SaveTokenError(sqlx::Error);

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),

    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
}

impl std::fmt::Display for SaveTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A database error encountered while trying to save token")
    }
}

impl std::fmt::Debug for SaveTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SaveTokenError {}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

impl std::error::Error for SaveTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_, _) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
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
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber = form.0.try_into()?;

    let token = generate_subscription_token();

    let mut transaction = db_pool.begin().await.map_err(|e| {
        SubscribeError::UnexpectedError(Box::new(e), "Failed to aquire transaction".into())
    })?;

    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction)
        .await
        .map_err(|e| {
            SubscribeError::UnexpectedError(Box::new(e), "Failed to insert subscriber".into())
        })?;

    save_token(subscriber_id, &token, &mut transaction)
        .await
        .map_err(|e| SubscribeError::UnexpectedError(Box::new(e), "Failed to save token".into()))?;

    let confirmation_link = generate_confirmation_link(&base_url.0, &token);

    transaction.commit().await.map_err(|e| {
        SubscribeError::UnexpectedError(Box::new(e), "Failed to commit transaction".into())
    })?;

    send_confirmation_link(
        email_client.as_ref(),
        &new_subscriber.email,
        confirmation_link,
    )
    .await
    .map_err(|e| {
        SubscribeError::UnexpectedError(Box::new(e), "Failed to send confirmation link".into())
    })?;

    tracing::info!("Subscriber save success");
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving subscriber in db"
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<uuid::Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO subscriptions(id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, $5)
    "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        "PENDING_CONFIRMATION",
    )
    .execute(transaction.deref_mut())
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {}", e);
        e
    })?;

    Ok(subscriber_id)
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

#[tracing::instrument(
    name = "Generating confirmation link"
    skip(token)
)]
fn generate_confirmation_link(base_url: &str, token: &secrecy::Secret<String>) -> String {
    format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        token.expose_secret()
    )
}

fn generate_subscription_token() -> secrecy::Secret<String> {
    let mut rng = thread_rng();
    let token = std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect();

    secrecy::Secret::new(token)
}

#[tracing::instrument(
    name = "Saving token in db"
    skip(subscription_token, transaction)
)]
pub async fn save_token(
    subscriber_id: uuid::Uuid,
    subscription_token: &secrecy::Secret<String>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SaveTokenError> {
    sqlx::query!(
        r#"
    INSERT INTO subscription_tokens(subscription_token, subscriber_id)
    VALUES ($1, $2)
    "#,
        subscription_token.expose_secret(),
        subscriber_id,
    )
    .execute(transaction.deref_mut())
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {}", e);
        SaveTokenError(e)
    })?;

    Ok(())
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused By:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}
