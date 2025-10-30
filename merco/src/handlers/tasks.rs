use crate::app::AppState;
use crate::errors::{ApiResult, AppError};
use crate::models::Timeframe;
use crate::tasks::types::{TaskContext, TaskStatus};
use crate::tasks::{Task, TaskConfig};
use axum::{
    extract::{Path, State},
    response::{
        Json,
        sse::{Event, KeepAlive, Sse},
    },
};
use chrono::{DateTime, Utc, serde::ts_milliseconds_option};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateFetchTaskRequest {
    pub symbol: String,
    pub exchange: String,
    pub timeframe: Timeframe,
    #[serde(default, with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub start: Option<DateTime<Utc>>,
    #[serde(default, with = "ts_milliseconds_option")]
    #[ts(optional, type = "number")]
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CreateTaskResponse {
    pub task_id: Uuid,
    pub status: TaskStatus,
}

pub async fn create_fetch_task(
    State(state): State<AppState>,
    Json(request): Json<CreateFetchTaskRequest>,
) -> ApiResult<CreateTaskResponse> {
    let context = TaskContext {
        db_pool: state.db_pool,
    };

    let config = TaskConfig::FetchCandles {
        symbol: request.symbol,
        exchange: request.exchange,
        timeframe: request.timeframe,
        start_date: request.start,
        end_date: request.end,
    };

    let task_id = state.task_manager.create_task(context, config).await;

    Ok(Json(CreateTaskResponse {
        task_id,
        status: TaskStatus::Pending,
    }))
}

pub async fn get_all_tasks(State(state): State<AppState>) -> ApiResult<Vec<Task>> {
    let tasks = state.task_manager.get_all_tasks().await;
    Ok(Json(tasks))
}

pub async fn get_task(State(state): State<AppState>, Path(task_id): Path<Uuid>) -> ApiResult<Task> {
    let task = state
        .task_manager
        .get_task(&task_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Task with id '{}' not found", task_id)))?;
    Ok(Json(task))
}

pub async fn stream_tasks(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.task_manager.subscribe();
    let initial_tasks = state.task_manager.get_all_tasks().await;

    let stream = async_stream::stream! {
        for task in initial_tasks {
            let event = crate::tasks::TaskEvent::Create { task };
            if let Ok(data) = serde_json::to_string(&event) {
                yield Ok(Event::default().data(data));
            }
        }

        loop {
            tokio::select! {
                _ = state.shutdown_token.cancelled() => {
                    break;
                }
                result = rx.recv() => {
                    let Ok(event) = result else {
                        break;
                    };

                    let Ok(data) = serde_json::to_string(&event) else {
                        continue;
                    };

                    yield Ok(Event::default().data(data));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
