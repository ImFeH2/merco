use super::types::{Task, TaskConfig, TaskEvent, TaskStatus};
use crate::errors::{AppError, AppResult};
use crate::exchange::ccxt::CCXT;
use crate::models::Timeframe;
use crate::services::candles;
use crate::tasks::types::TaskContext;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
    event_tx: broadcast::Sender<TaskEvent>,
}

impl TaskManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    pub async fn create_task(&self, context: TaskContext, config: TaskConfig) -> Uuid {
        let task = Task::new(config.clone());
        let task_id = task.id;

        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id, task.clone());

        let _ = self.event_tx.send(TaskEvent::Create { task });
        drop(tasks);

        let manager = self.clone();
        tokio::spawn(async move {
            manager.execute_task(task_id, context, config).await;
        });

        task_id
    }

    async fn execute_task(&self, task_id: Uuid, context: TaskContext, config: TaskConfig) {
        self.update_status(task_id, TaskStatus::Running).await;

        let result = match config {
            TaskConfig::FetchCandles {
                symbol,
                exchange,
                timeframe,
                start_date,
                end_date,
            } => {
                self.fetch_candles_data(
                    task_id, context, &symbol, &exchange, timeframe, start_date, end_date,
                )
                .await
            }
        };

        match result {
            Ok(data) => {
                self.complete_task(task_id, Some(data)).await;
            }
            Err(e) => {
                self.fail_task(task_id, e).await;
            }
        }
    }

    async fn fetch_candles_data(
        &self,
        task_id: Uuid,
        context: TaskContext,
        symbol: &str,
        exchange: &str,
        timeframe: Timeframe,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> AppResult<serde_json::Value> {
        tracing::info!(
            "Fetching candles data for {} on {} with timeframe {}",
            symbol,
            exchange,
            timeframe
        );

        let ccxt = CCXT::try_from_exchange(exchange)?;
        let pool = context.db_pool;

        let timeframe_ms = timeframe.to_ms();
        let timeframe_delta = timeframe.to_delta();
        let mut next_since =
            match candles::get_latest_candle(&pool, exchange, symbol, timeframe).await? {
                Some(latest_candle) => latest_candle.timestamp + timeframe_delta,
                None => {
                    let first_batch = ccxt.fetch_candles(symbol, timeframe, Some(0), None)?;
                    let Some(latest_candle) = first_batch.last() else {
                        return Err(format!(
                            "No candles data available for {} on {}",
                            symbol, exchange
                        )
                        .into());
                    };

                    candles::insert_candles(&pool, &first_batch).await?;

                    let latest = latest_candle.timestamp;
                    latest + timeframe_delta
                }
            };

        let now = Utc::now();
        let duration = now.signed_duration_since(next_since);
        let Some(time_diff_ms) = duration.num_milliseconds().to_u64() else {
            return Err(format!("Invalid time range for {} on {}", symbol, exchange).into());
        };

        let mut count: u64 = 0;
        let total = (time_diff_ms + timeframe_ms - 1) / timeframe_ms;
        let mut progress = 0.0;
        self.update_progress(task_id, progress).await;

        loop {
            let next_since_ms = next_since.timestamp_millis() as u64;
            let batch = ccxt.fetch_candles(symbol, timeframe, Some(next_since_ms), None)?;

            let Some(latest) = batch.last() else {
                break;
            };

            candles::insert_candles(&pool, &batch).await?;

            next_since = latest.timestamp + timeframe_delta;
            count += batch.len() as u64;
            progress = 100.0 * (count as f32) / (total as f32);
            self.update_progress(task_id, progress).await;
        }

        Ok(serde_json::json!({
            "symbol": symbol,
            "exchange": exchange,
            "timeframe": timeframe,
            "start_date": start_date,
            "end_date": end_date,
            "records": total,
        }))
    }

    async fn update_progress(&self, task_id: Uuid, progress: f32) {
        let now = Utc::now();
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.progress = progress;
            task.updated_at = now;

            let _ = self.event_tx.send(TaskEvent::Progress {
                task_id,
                progress,
                status: task.status.clone(),
            });
        }
    }

    async fn update_status(&self, task_id: Uuid, status: TaskStatus) {
        let now = Utc::now();
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = status.clone();
            task.updated_at = now;

            if status == TaskStatus::Running && task.started_at.is_none() {
                task.started_at = Some(now);
            }

            let _ = self.event_tx.send(TaskEvent::Status { task_id, status });
        }
    }

    async fn complete_task(&self, task_id: Uuid, result: Option<serde_json::Value>) {
        let now = Utc::now();
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
            task.progress = 100.0;
            task.result = result.clone();
            task.completed_at = Some(now);
            task.updated_at = now;

            let _ = self.event_tx.send(TaskEvent::Complete { task_id, result });
        }
    }

    async fn fail_task(&self, task_id: Uuid, error: AppError) {
        let now = Utc::now();
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = TaskStatus::Failed;
            task.error_message = Some(error.to_string());
            task.completed_at = Some(now);
            task.updated_at = now;

            let _ = self.event_tx.send(TaskEvent::Fail {
                task_id,
                error: error.to_string(),
            });
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_tx.subscribe()
    }

    pub async fn get_all_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    pub async fn get_task(&self, task_id: &Uuid) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
