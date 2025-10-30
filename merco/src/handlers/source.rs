use std::path::PathBuf;

use crate::{
    errors::{ApiResult, AppError},
    strategy::STRATEGY_WORKDIR_NAME,
    utils::safe_join,
};
use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use tokio::fs;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct GetSourceQuery {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export, tag = "type")]
pub enum GetSourceResponse {
    File {
        name: String,
        path: String,
        content: String,
    },
    Directory {
        name: String,
        path: String,
        children: Vec<FileNode>,
    },
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: FileNodeType,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FileNodeType {
    File,
    Directory,
}

pub async fn get_source(Query(query): Query<GetSourceQuery>) -> ApiResult<GetSourceResponse> {
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join(STRATEGY_WORKDIR_NAME).canonicalize()?;
    let full_path = safe_join(&base_dir, &query.path)?;

    let Ok(relative_path) = full_path.strip_prefix(&base_dir) else {
        return Err(AppError::BadRequest(
            "Access denied: path outside workspace".to_string(),
        ));
    };

    let name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    let metadata = fs::metadata(&full_path).await?;

    if metadata.is_dir() {
        let mut children = Vec::new();
        let mut read_dir = fs::read_dir(&full_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let entry_type = entry.file_type().await?;
            let entry_name = entry.file_name().to_str().unwrap_or_default().to_string();
            let entry_path = relative_path
                .join(&entry_name)
                .to_string_lossy()
                .to_string();

            let node_type = if entry_type.is_dir() {
                FileNodeType::Directory
            } else if entry_type.is_file() {
                FileNodeType::File
            } else {
                continue;
            };

            children.push(FileNode {
                name: entry_name,
                path: entry_path,
                node_type,
            });
        }

        children.sort_by(|a, b| match (&a.node_type, &b.node_type) {
            (FileNodeType::Directory { .. }, FileNodeType::File { .. }) => std::cmp::Ordering::Less,
            (FileNodeType::File { .. }, FileNodeType::Directory { .. }) => {
                std::cmp::Ordering::Greater
            }
            _ => a.name.cmp(&b.name),
        });

        Ok(Json(GetSourceResponse::Directory {
            name,
            path: relative_path.to_string_lossy().to_string(),
            children,
        }))
    } else if metadata.is_file() {
        let content = fs::read_to_string(&full_path).await?;

        Ok(Json(GetSourceResponse::File {
            name: full_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
            path: relative_path.to_string_lossy().to_string(),
            content,
        }))
    } else {
        Err(AppError::BadRequest("Unsupported file type".to_string()))
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SaveSourceQuery {
    pub path: String,
}

pub async fn save_source(
    Query(query): Query<SaveSourceQuery>,
    Json(content): Json<String>,
) -> ApiResult<()> {
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join(STRATEGY_WORKDIR_NAME).canonicalize()?;
    let full_path = safe_join(&base_dir, &query.path)?;

    if full_path.exists() {
        let metadata = fs::metadata(&full_path).await?;
        if !metadata.is_file() {
            return Err(AppError::BadRequest("Can only save to files".to_string()));
        }
    }

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(&full_path, content).await?;
    Ok(Json(()))
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct DeleteSourceQuery {
    pub path: String,
}

pub async fn delete_source(Query(query): Query<DeleteSourceQuery>) -> ApiResult<()> {
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join(STRATEGY_WORKDIR_NAME).canonicalize()?;
    let full_path = safe_join(&base_dir, &query.path)?;

    if !full_path.exists() {
        return Err(AppError::NotFound("Path does not exist".to_string()));
    }

    if full_path == base_dir {
        return Err(AppError::BadRequest(
            "Cannot delete the root workspace directory".to_string(),
        ));
    }

    let metadata = fs::metadata(&full_path).await?;
    if metadata.is_dir() {
        fs::remove_dir_all(&full_path).await?;
    } else if metadata.is_file() {
        fs::remove_file(&full_path).await?;
    }

    Ok(Json(()))
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct MoveSourceQuery {
    pub old_path: String,
    pub new_path: String,
}

pub async fn move_source(Query(query): Query<MoveSourceQuery>) -> ApiResult<()> {
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join(STRATEGY_WORKDIR_NAME).canonicalize()?;
    let full_old_path = safe_join(&base_dir, &query.old_path)?;
    let full_new_path = safe_join(&base_dir, &query.new_path)?;

    if !full_old_path.exists() {
        return Err(AppError::NotFound("Path does not exist".to_string()));
    }

    if full_old_path == base_dir {
        return Err(AppError::BadRequest(
            "Cannot move the root workspace directory".to_string(),
        ));
    }

    if full_new_path.exists() {
        return Err(AppError::BadRequest(
            "Destination path already exists".to_string(),
        ));
    }

    fs::rename(&full_old_path, &full_new_path).await?;
    Ok(Json(()))
}
