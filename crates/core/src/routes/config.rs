use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
    Json, Router,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use services::services::config::{Config, EditorConstants, ExecutorConfig, SoundConstants};
use tokio::fs;
use ts_rs::TS;
use utils::{assets::config_path, response::ApiResponse};

use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/info", get(get_user_system_info))
        .route("/config", post(update_config))
        .route("/mcp-config", get(get_mcp_servers))
        .route("/mcp-config", post(update_mcp_servers))
}

#[derive(Debug, Serialize, Deserialize, TS)]
struct Environment {
    pub os_type: String,
    pub os_version: String,
    pub os_architecture: String,
    pub bitness: String,
}

impl Environment {
    pub fn new() -> Self {
        let info = os_info::get();
        Environment {
            os_type: info.os_type().to_string(),
            os_version: info.version().to_string(),
            os_architecture: info.architecture().unwrap_or("unknown").to_string(),
            bitness: info.bitness().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UserSystemInfo {
    pub config: Config,
    pub environment: Environment,
    pub editor_config_options: EditorConstants,
    pub sound_config_options: SoundConstants,
}

// TODO: update frontend, BE schema has changed, this replaces GET /config and /config/constants
#[axum::debug_handler]
async fn get_user_system_info(
    State(deployment): State<DeploymentImpl>,
) -> ResponseJson<ApiResponse<UserSystemInfo>> {
    let config = deployment.config().read().await;

    let user_system_info = UserSystemInfo {
        config: config.clone(),
        environment: Environment::new(),
        editor_config_options: EditorConstants::new(),
        sound_config_options: SoundConstants::new(),
    };

    ResponseJson(ApiResponse::success(user_system_info))
}

async fn update_config(
    State(deployment): State<DeploymentImpl>,
    Json(new_config): Json<Config>,
) -> ResponseJson<ApiResponse<Config>> {
    let config_path = config_path();

    match new_config.save(&config_path) {
        Ok(_) => {
            let mut config = deployment.config().write().await;
            *config = new_config.clone();
            drop(config);

            ResponseJson(ApiResponse::success(new_config))
        }
        Err(e) => ResponseJson(ApiResponse::error(&format!("Failed to save config: {}", e))),
    }
}

#[derive(Debug, Deserialize)]
struct McpServerQuery {
    executor: Option<String>,
}

/// Common logic for resolving executor configuration and validating MCP support
fn resolve_executor_config(
    query_executor: Option<String>,
    saved_config: &ExecutorConfig,
) -> Result<ExecutorConfig, String> {
    let executor_config = match query_executor {
        Some(executor_type) => executor_type
            .parse::<ExecutorConfig>()
            .map_err(|e| e.to_string())?,
        None => saved_config.clone(),
    };

    if executor_config.mcp_attribute_path().is_none() {
        return Err(format!(
            "{executor_config} executor does not support MCP configuration",
        ));
    }

    Ok(executor_config)
}

async fn get_mcp_servers(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<McpServerQuery>,
) -> ResponseJson<ApiResponse<Value>> {
    let saved_config = {
        let config = deployment.config().read().await;
        config.executor.clone()
    };

    let executor_config = match resolve_executor_config(query.executor, &saved_config) {
        Ok(config) => config,
        Err(message) => {
            return ResponseJson(ApiResponse::error(&message));
        }
    };

    // Get the config file path for this executor
    let config_path = match executor_config.config_path() {
        Some(path) => path,
        None => {
            return ResponseJson(ApiResponse::error("Could not determine config file path"));
        }
    };

    match read_mcp_servers_from_config(&config_path, &executor_config).await {
        Ok(servers) => {
            let response_data = serde_json::json!({
                "servers": servers,
                "config_path": config_path.to_string_lossy().to_string()
            });
            ResponseJson(ApiResponse::success(response_data))
        }
        Err(e) => ResponseJson(ApiResponse::error(&format!(
            "Failed to read MCP servers: {}",
            e
        ))),
    }
}

async fn update_mcp_servers(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<McpServerQuery>,
    Json(new_servers): Json<HashMap<String, Value>>,
) -> ResponseJson<ApiResponse<String>> {
    let saved_config = {
        let config = deployment.config().read().await;
        config.executor.clone()
    };

    let executor_config = match resolve_executor_config(query.executor, &saved_config) {
        Ok(config) => config,
        Err(message) => {
            return ResponseJson(ApiResponse::error(&message));
        }
    };

    // Get the config file path for this executor
    let config_path = match executor_config.config_path() {
        Some(path) => path,
        None => {
            return ResponseJson(ApiResponse::error("Could not determine config file path"));
        }
    };

    match update_mcp_servers_in_config(&config_path, &executor_config, new_servers).await {
        Ok(message) => ResponseJson(ApiResponse::success(message)),
        Err(e) => ResponseJson(ApiResponse::error(&format!(
            "Failed to update MCP servers: {}",
            e
        ))),
    }
}

async fn update_mcp_servers_in_config(
    file_path: &std::path::Path,
    executor_config: &ExecutorConfig,
    new_servers: HashMap<String, Value>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Read existing config file or create empty object if it doesn't exist
    let file_content = fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let mut config: Value = serde_json::from_str(&file_content)?;

    // Get the attribute path for MCP servers
    let mcp_path = executor_config.mcp_attribute_path().unwrap();

    // Get the current server count for comparison
    let old_servers = get_mcp_servers_from_config_path(executor_config, &config, &mcp_path).len();

    // Set the MCP servers using the correct attribute path
    set_mcp_servers_in_config_path(executor_config, &mut config, &mcp_path, &new_servers)?;

    // Write the updated config back to file
    let updated_content = serde_json::to_string_pretty(&config)?;
    fs::write(file_path, updated_content).await?;

    let new_count = new_servers.len();
    let message = match (old_servers, new_count) {
        (0, 0) => "No MCP servers configured".to_string(),
        (0, n) => format!("Added {} MCP server(s)", n),
        (old, new) if old == new => format!("Updated MCP server configuration ({} server(s))", new),
        (old, new) => format!(
            "Updated MCP server configuration (was {}, now {})",
            old, new
        ),
    };

    Ok(message)
}

async fn read_mcp_servers_from_config(
    file_path: &std::path::Path,
    executor_config: &ExecutorConfig,
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
    // Read the config file, return empty if it doesn't exist
    let file_content = fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "{}".to_string());
    let raw_config: Value = serde_json::from_str(&file_content)?;

    // Get the attribute path for MCP servers
    let mcp_path = executor_config.mcp_attribute_path().unwrap();

    // Get the servers using the correct attribute path
    let servers = get_mcp_servers_from_config_path(&executor_config, &raw_config, &mcp_path);

    Ok(servers)
}

/// Helper function to get MCP servers from config using a path
fn get_mcp_servers_from_config_path(
    executor_config: &ExecutorConfig,
    raw_config: &Value,
    path: &[&str],
) -> HashMap<String, Value> {
    // Special handling for AMP - use flat key structure
    if matches!(executor_config, ExecutorConfig::Amp) {
        let flat_key = format!("{}.{}", path[0], path[1]);
        let current = match raw_config.get(&flat_key) {
            Some(val) => val,
            None => return HashMap::new(),
        };

        // Extract the servers object
        match current.as_object() {
            Some(servers) => servers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => HashMap::new(),
        }
    } else {
        let mut current = raw_config;

        // Navigate to the target location
        for &part in path {
            current = match current.get(part) {
                Some(val) => val,
                None => return HashMap::new(),
            };
        }

        // Extract the servers object
        match current.as_object() {
            Some(servers) => servers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => HashMap::new(),
        }
    }
}

/// Helper function to set MCP servers in config using a path
fn set_mcp_servers_in_config_path(
    executor_config: &ExecutorConfig,
    raw_config: &mut Value,
    path: &[&str],
    servers: &HashMap<String, Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure config is an object
    if !raw_config.is_object() {
        *raw_config = serde_json::json!({});
    }

    // Special handling for AMP - use flat key structure
    if matches!(executor_config, ExecutorConfig::Amp) {
        let flat_key = format!("{}.{}", path[0], path[1]);
        raw_config
            .as_object_mut()
            .unwrap()
            .insert(flat_key, serde_json::to_value(servers)?);
        return Ok(());
    }

    let mut current = raw_config;

    // Navigate/create the nested structure (all parts except the last)
    for &part in &path[..path.len() - 1] {
        if current.get(part).is_none() {
            current
                .as_object_mut()
                .unwrap()
                .insert(part.to_string(), serde_json::json!({}));
        }
        current = current.get_mut(part).unwrap();
        if !current.is_object() {
            *current = serde_json::json!({});
        }
    }

    // Set the final attribute
    let final_attr = path.last().unwrap();
    current
        .as_object_mut()
        .unwrap()
        .insert(final_attr.to_string(), serde_json::to_value(servers)?);

    Ok(())
}
