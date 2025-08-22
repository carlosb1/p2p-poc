use axum::{routing::post, Router, Json, extract::Extension};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use redis::AsyncCommands;

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub payload: String,
}

pub async fn push_task(
    task: Task,
    redis_client: Arc<redis::Client>,
) -> anyhow::Result<()> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    let task_json = serde_json::to_string(&task)?;
    let _: () = conn.rpush("task_queue", task_json).await?;
    Ok(())
}