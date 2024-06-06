use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient};

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

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(&self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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

#[tracing::instrument(
    name = "Publish Newsletter to subscriber",
    skip(body, db_pool, email_client)
)]
pub async fn publish_newsletter(
    body: web::Json<PublishNLBody>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .with_context(|| "Failed to get subscribers from db")?;

    for subscriber in subscribers {
        email_client
            .send_email(
                &subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .with_context(|| format!("Failed to send email: {}", subscriber.email.as_ref()))?;
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscriber list", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, sqlx::Error> {
    struct Row {
        email: String,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT email FROM subscriptions WHERE status = 'CONFIRMED'"#
    )
    .fetch_all(db_pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| ConfirmedSubscriber {
            email: SubscriberEmail::parse(r.email).unwrap(),
        })
        .collect();

    Ok(confirmed_subscribers)
}
