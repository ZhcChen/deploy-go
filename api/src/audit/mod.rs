use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use ulid::Ulid;

pub async fn record(
    transaction: &mut Transaction<'_, Sqlite>,
    actor_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    request_id: &str,
    summary: Value,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (id, actor_id, action, resource_type, resource_id, request_id, summary_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(format!("aud_{}", Ulid::new()))
    .bind(actor_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(request_id)
    .bind(summary.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
