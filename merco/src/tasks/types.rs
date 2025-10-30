use chrono::{serde::ts_milliseconds, serde::ts_milliseconds_option, DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use uuid::Uuid;

use crate::models::Timeframe;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TaskType {
    FetchCandles,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export, tag = "type")]
pub enum TaskConfig {
    FetchCandles {
        symbol: String,
        exchange: String,
        timeframe: Timeframe,
        #[serde(with = "ts_milliseconds_option")]
        #[ts(optional, type = "number")]
        start_date: Option<DateTime<Utc>>,
        #[serde(with = "ts_milliseconds_option")]
        #[ts(optional, type = "number")]
        end_date: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Task {
    pub id: Uuid,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub progress: f32,
    pub config: TaskConfig,
    #[ts(optional)]
    pub result: Option<serde_json::Value>,
    #[ts(optional)]
    pub error_message: Option<String>,
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    #[ts(type = "number")]
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(config: TaskConfig) -> Self {
        let now = Utc::now();
        let task_type = match &config {
            TaskConfig::FetchCandles { .. } => TaskType::FetchCandles,
        };

        Self {
            id: Uuid::new_v4(),
            task_type,
            status: TaskStatus::Pending,
            progress: 0.0,
            config,
            result: None,
            error_message: None,
            created_at: now,
            started_at: None,
            completed_at: None,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export, tag = "type")]
pub enum TaskEvent {
    Create {
        task: Task,
    },
    Progress {
        task_id: Uuid,
        progress: f32,
        status: TaskStatus,
    },
    Status {
        task_id: Uuid,
        status: TaskStatus,
    },
    Complete {
        task_id: Uuid,
        #[ts(optional)]
        result: Option<serde_json::Value>,
    },
    Fail {
        task_id: Uuid,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub db_pool: PgPool,
}
