use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubConfirmationParam {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm pending subscriber", skip(param, db_pool))]
pub async fn confirm(
    param: web::Query<SubConfirmationParam>,
    db_pool: web::Data<PgPool>,
) -> HttpResponse {
    tracing::trace!("Parameters recieved: {:?}", param.subscription_token);

    let subscriber_id =
        match get_subscriber_id_from_token(&db_pool, &param.subscription_token).await {
            Ok(id) => id,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    if subscriber_id.is_none() {
        return HttpResponse::NotFound().finish();
    }

    if set_subscriber_status_to_confirmed(&db_pool, subscriber_id.unwrap())
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Get subscriber id from token", skip(db_pool, token))]
async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        token
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Set subscriber status to confirmed", skip(db_pool))]
async fn set_subscriber_status_to_confirmed(
    db_pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'CONFIRMED' WHERE id = $1"#,
        subscriber_id
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
