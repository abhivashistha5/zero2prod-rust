use actix_web::{
    http::{
        header::{self, HeaderMap, HeaderValue},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
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

struct Credentials {
    username: String,
    password: secrecy::Secret<String>,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication Failed")]
    AuthError(#[source] anyhow::Error),

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
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
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

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("'Authorization' header missing")?
        .to_str()
        .context("'Authorization' header was not a valid utf-8 string")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The Authorization scheme was not Basic")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to decode base64 credentials")?;

    let decoded_credentials =
        String::from_utf8(decoded_bytes).context("The decoded credentials is not valid utf-8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Username must be provided"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("Password must be provided"))?
        .to_string();

    Ok(Credentials {
        username,
        password: secrecy::Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, db_pool))]
async fn validate_credentials(
    credentials: Credentials,
    db_pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials.username, db_pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))?;

    tokio::task::spawn_blocking(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    // spawn_blocking is fallible - we have a nested Result here!
    .context("Failed to spawn blocking task.")
    .map_err(PublishError::UnexpectedError)?
    .context("Invalid password.")
    .map_err(PublishError::AuthError)?;

    Ok(user_id)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid Password.")
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, db_pool))]
async fn get_stored_credentials(
    username: &str,
    db_pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
    SELECT user_id, password_hash
    FROM users
    WHERE username = $1
    "#,
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to fetch credentials from database")?
    .map(|r| (r.user_id, Secret::new(r.password_hash)));

    Ok(row)
}

#[tracing::instrument(
    name = "Publish Newsletter to subscriber",
    skip(body, db_pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<PublishNLBody>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &db_pool).await?;

    tracing::Span::current().record("username", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .with_context(|| "Failed to get subscribers from db")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(s) => email_client
                .send_email(
                    &s.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| format!("Failed to send newsletter: {}", s.email))?,
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping Confirmed Subscriber. Invalid data stored")
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscriber list", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'CONFIRMED'"#)
            .fetch_all(db_pool)
            .await?
            .into_iter()
            .map(|r| match SubscriberEmail::parse(r.email) {
                Ok(v) => Ok(ConfirmedSubscriber { email: v }),
                Err(e) => Err(anyhow::anyhow!(e)),
            })
            .collect();

    Ok(confirmed_subscribers)
}
