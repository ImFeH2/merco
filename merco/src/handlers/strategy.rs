use crate::{app::AppState, errors::ApiResult};
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use std::fs;
use toml_edit::DocumentMut;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct AddStrategyRequest {
    pub name: String,
}

pub async fn add_strategy(
    State(state): State<AppState>,
    Json(request): Json<AddStrategyRequest>,
) -> ApiResult<()> {
    let strategy_manager = state.strategy_manager;
    strategy_manager.add_strategy(&request.name)?;

    Ok(Json(()))
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ListStrategiesResponse {
    pub strategies: Vec<String>,
}

pub async fn list_strategies() -> ApiResult<ListStrategiesResponse> {
    let current_dir = std::env::current_dir().map_err(|e| {
        crate::errors::AppError::Internal(format!("Failed to get current dir: {}", e))
    })?;
    let workspace_toml_path = current_dir.join("strategies").join("Cargo.toml");
    let content = fs::read_to_string(&workspace_toml_path).map_err(|e| {
        crate::errors::AppError::Internal(format!(
            "Failed to read Cargo.toml at {:?}: {}",
            workspace_toml_path, e
        ))
    })?;

    let doc: DocumentMut = content.parse().map_err(|e| {
        crate::errors::AppError::Internal(format!("Failed to parse Cargo.toml: {}", e))
    })?;

    let members = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| {
            crate::errors::AppError::Internal(
                "No workspace.members found in Cargo.toml".to_string(),
            )
        })?;

    let strategies: Vec<String> = members
        .iter()
        .filter_map(|m| m.as_str().map(|s| s.to_string()))
        .collect();

    Ok(Json(ListStrategiesResponse { strategies }))
}
