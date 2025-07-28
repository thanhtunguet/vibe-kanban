use std::{fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::{assets::SoundAssets, cache_dir};

// use crate::{SoundAssets, executor::ExecutorConfig};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvironmentInfo {
    pub os_type: String,
    pub os_version: String,
    pub architecture: String,
    pub bitness: String,
}

/// Configuration for different executor types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ExecutorConfig {
    Echo,
    Claude,
    /// TODO: Claude Plan should share configuration with Claude
    ClaudePlan,
    Amp,
    Gemini,
    #[serde(alias = "setup_script")]
    SetupScript {
        script: String,
    },
    ClaudeCodeRouter,
    #[serde(alias = "charmopencode")]
    CharmOpencode,
    #[serde(alias = "opencode")]
    SstOpencode,
    Aider,
    Codex,
}

impl ExecutorConfig {
    /// Cheap, non-allocating view of the "type" string.
    pub fn as_type_str(&self) -> &'static str {
        match self {
            ExecutorConfig::Echo => "echo",
            ExecutorConfig::Claude => "claude",
            ExecutorConfig::ClaudePlan => "claude-plan",
            ExecutorConfig::Amp => "amp",
            ExecutorConfig::Gemini => "gemini",
            ExecutorConfig::SetupScript { .. } => "setup-script",
            ExecutorConfig::ClaudeCodeRouter => "claude-code-router",
            ExecutorConfig::CharmOpencode => "charm-opencode",
            ExecutorConfig::SstOpencode => "sst-opencode",
            ExecutorConfig::Aider => "aider",
            ExecutorConfig::Codex => "codex",
        }
    }

    pub fn config_path(&self) -> Option<PathBuf> {
        match self {
            ExecutorConfig::Echo => None,
            ExecutorConfig::CharmOpencode => {
                dirs::home_dir().map(|home| home.join(".opencode.json"))
            }
            ExecutorConfig::Claude => dirs::home_dir().map(|home| home.join(".claude.json")),
            ExecutorConfig::ClaudePlan => dirs::home_dir().map(|home| home.join(".claude.json")),
            ExecutorConfig::ClaudeCodeRouter => {
                dirs::home_dir().map(|home| home.join(".claude.json"))
            }
            ExecutorConfig::Amp => {
                dirs::config_dir().map(|config| config.join("amp").join("settings.json"))
            }
            ExecutorConfig::Gemini => {
                dirs::home_dir().map(|home| home.join(".gemini").join("settings.json"))
            }
            ExecutorConfig::SstOpencode => {
                #[cfg(unix)]
                {
                    xdg::BaseDirectories::with_prefix("opencode").get_config_file("opencode.json")
                }
                #[cfg(not(unix))]
                {
                    dirs::config_dir().map(|config| config.join("opencode").join("opencode.json"))
                }
            }
            ExecutorConfig::Aider => None,
            ExecutorConfig::Codex => {
                dirs::home_dir().map(|home| home.join(".codex").join("config.toml"))
            }
            ExecutorConfig::SetupScript { .. } => None,
        }
    }

    /// Get the JSON attribute path for MCP servers in the config file
    /// Returns None if the executor doesn't support MCP
    pub fn mcp_attribute_path(&self) -> Option<Vec<&'static str>> {
        match self {
            ExecutorConfig::Echo => None, // Echo doesn't support MCP
            ExecutorConfig::CharmOpencode => Some(vec!["mcpServers"]),
            ExecutorConfig::SstOpencode => Some(vec!["mcp"]),
            ExecutorConfig::Claude => Some(vec!["mcpServers"]),
            ExecutorConfig::ClaudePlan => None, // Claude Plan shares Claude config
            ExecutorConfig::Amp => Some(vec!["amp", "mcpServers"]), // Nested path for Amp
            ExecutorConfig::Gemini => Some(vec!["mcpServers"]),
            ExecutorConfig::ClaudeCodeRouter => Some(vec!["mcpServers"]),
            ExecutorConfig::Aider => None, // Aider doesn't support MCP. https://github.com/Aider-AI/aider/issues/3314
            ExecutorConfig::Codex => None, // Codex uses TOML config, frontend doesn't handle TOML yet
            ExecutorConfig::SetupScript { .. } => None, // Setup scripts don't support MCP
        }
    }
}

impl fmt::Display for ExecutorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_type_str())
    }
}

impl FromStr for ExecutorConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "echo" => Ok(ExecutorConfig::Echo),
            "claude" => Ok(ExecutorConfig::Claude),
            "claude-plan" => Ok(ExecutorConfig::ClaudePlan),
            "amp" => Ok(ExecutorConfig::Amp),
            "gemini" => Ok(ExecutorConfig::Gemini),
            "charm-opencode" => Ok(ExecutorConfig::CharmOpencode),
            "claude-code-router" => Ok(ExecutorConfig::ClaudeCodeRouter),
            "sst-opencode" => Ok(ExecutorConfig::SstOpencode),
            "aider" => Ok(ExecutorConfig::Aider),
            "codex" => Ok(ExecutorConfig::Codex),
            "setup-script" => Ok(ExecutorConfig::SetupScript {
                script: "setup script".to_string(),
            }),
            _ => Err(format!("Unknown executor type: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Config {
    pub theme: ThemeMode,
    pub executor: ExecutorConfig,
    pub disclaimer_acknowledged: bool,
    pub onboarding_acknowledged: bool,
    pub github_login_acknowledged: bool,
    pub telemetry_acknowledged: bool,
    pub sound_alerts: bool,
    pub sound_file: SoundFile,
    pub push_notifications: bool,
    pub editor: EditorConfig,
    pub github: GitHubConfig,
    pub analytics_enabled: Option<bool>,
    pub environment: EnvironmentInfo,
    pub workspace_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
    Purple,
    Green,
    Blue,
    Orange,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditorConfig {
    pub editor_type: EditorType,
    pub custom_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GitHubConfig {
    pub pat: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub primary_email: Option<String>,
    pub default_pr_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum EditorType {
    VSCode,
    Cursor,
    Windsurf,
    IntelliJ,
    Zed,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "kebab-case")]
pub enum SoundFile {
    AbstractSound1,
    AbstractSound2,
    AbstractSound3,
    AbstractSound4,
    CowMooing,
    PhoneVibration,
    Rooster,
}

// Constants for frontend
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditorConstants {
    pub editor_types: Vec<EditorType>,
    pub editor_labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SoundConstants {
    pub sound_files: Vec<SoundFile>,
    pub sound_labels: Vec<String>,
}

impl EditorConstants {
    pub fn new() -> Self {
        Self {
            editor_types: vec![
                EditorType::VSCode,
                EditorType::Cursor,
                EditorType::Windsurf,
                EditorType::IntelliJ,
                EditorType::Zed,
                EditorType::Custom,
            ],
            editor_labels: vec![
                "VS Code".to_string(),
                "Cursor".to_string(),
                "Windsurf".to_string(),
                "IntelliJ IDEA".to_string(),
                "Zed".to_string(),
                "Custom".to_string(),
            ],
        }
    }
}

impl Default for EditorConstants {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundConstants {
    pub fn new() -> Self {
        Self {
            sound_files: vec![
                SoundFile::AbstractSound1,
                SoundFile::AbstractSound2,
                SoundFile::AbstractSound3,
                SoundFile::AbstractSound4,
                SoundFile::CowMooing,
                SoundFile::PhoneVibration,
                SoundFile::Rooster,
            ],
            sound_labels: vec![
                "Gentle Chime".to_string(),
                "Soft Bell".to_string(),
                "Digital Tone".to_string(),
                "Subtle Alert".to_string(),
                "Cow Mooing".to_string(),
                "Phone Vibration".to_string(),
                "Rooster Call".to_string(),
            ],
        }
    }
}

impl Default for SoundConstants {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Config {
    fn default() -> Self {
        let info = os_info::get();

        Self {
            theme: ThemeMode::System,
            // TODO: fix
            executor: ExecutorConfig::Claude,
            disclaimer_acknowledged: false,
            onboarding_acknowledged: false,
            github_login_acknowledged: false,
            telemetry_acknowledged: false,
            sound_alerts: true,
            sound_file: SoundFile::AbstractSound4,
            push_notifications: true,
            editor: EditorConfig::default(),
            github: GitHubConfig::default(),
            analytics_enabled: None,
            environment: EnvironmentInfo {
                os_type: info.os_type().to_string(),
                os_version: info.version().to_string(),
                architecture: info.architecture().unwrap_or("unknown").to_string(),
                bitness: info.bitness().to_string(),
            },
            workspace_dir: None,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            editor_type: EditorType::VSCode,
            custom_command: None,
        }
    }
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            pat: None,
            token: None,
            username: None,
            primary_email: None,
            default_pr_base: Some("main".to_string()),
        }
    }
}

impl EditorConfig {
    pub fn get_command(&self) -> Vec<String> {
        match &self.editor_type {
            EditorType::VSCode => vec!["code".to_string()],
            EditorType::Cursor => vec!["cursor".to_string()],
            EditorType::Windsurf => vec!["windsurf".to_string()],
            EditorType::IntelliJ => vec!["idea".to_string()],
            EditorType::Zed => vec!["zed".to_string()],
            EditorType::Custom => {
                if let Some(custom) = &self.custom_command {
                    custom.split_whitespace().map(|s| s.to_string()).collect()
                } else {
                    vec!["code".to_string()] // fallback to VSCode
                }
            }
        }
    }
}

impl SoundFile {
    pub fn to_filename(&self) -> &'static str {
        match self {
            SoundFile::AbstractSound1 => "abstract-sound1.wav",
            SoundFile::AbstractSound2 => "abstract-sound2.wav",
            SoundFile::AbstractSound3 => "abstract-sound3.wav",
            SoundFile::AbstractSound4 => "abstract-sound4.wav",
            SoundFile::CowMooing => "cow-mooing.wav",
            SoundFile::PhoneVibration => "phone-vibration.wav",
            SoundFile::Rooster => "rooster.wav",
        }
    }

    /// Get or create a cached sound file with the embedded sound data
    pub async fn get_path(&self) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        use std::io::Write;

        let filename = self.to_filename();
        let cache_dir = cache_dir();
        let cached_path = cache_dir.join(format!("sound-{}", filename));

        // Check if cached file already exists and is valid
        if cached_path.exists() {
            // Verify file has content (basic validation)
            if let Ok(metadata) = std::fs::metadata(&cached_path) {
                if metadata.len() > 0 {
                    return Ok(cached_path);
                }
            }
        }

        // File doesn't exist or is invalid, create it
        let sound_data = SoundAssets::get(filename)
            .ok_or_else(|| format!("Embedded sound file not found: {}", filename))?
            .data;

        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        let mut file = std::fs::File::create(&cached_path)
            .map_err(|e| format!("Failed to create cached sound file: {}", e))?;

        file.write_all(&sound_data)
            .map_err(|e| format!("Failed to write sound data to cached file: {}", e))?;

        drop(file); // Ensure file is closed

        Ok(cached_path)
    }
}

impl Config {
    pub fn load(config_path: &PathBuf) -> anyhow::Result<Self> {
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;

            // Try to deserialize as is first
            match serde_json::from_str::<Config>(&content) {
                Ok(mut config) => {
                    if config.analytics_enabled.is_none() {
                        config.analytics_enabled = Some(true);
                    }

                    // Always save back to ensure new fields are written to disk
                    config.save(config_path)?;
                    Ok(config)
                }
                Err(_) => {
                    // If full deserialization fails, try to merge with defaults
                    match Self::load_with_defaults(&content, config_path) {
                        Ok(config) => Ok(config),
                        Err(_) => {
                            // Even partial loading failed - backup the corrupted file
                            if let Err(e) = Self::backup_corrupted_config(config_path) {
                                tracing::error!("Failed to backup corrupted config: {}", e);
                            }

                            // Remove corrupted file and create a default config
                            if let Err(e) = std::fs::remove_file(config_path) {
                                tracing::error!("Failed to remove corrupted config file: {}", e);
                            }

                            // Create and save default config
                            let config = Config::default();
                            config.save(config_path)?;
                            Ok(config)
                        }
                    }
                }
            }
        } else {
            let config = Config::default();
            config.save(config_path)?;
            Ok(config)
        }
    }

    fn load_with_defaults(content: &str, config_path: &PathBuf) -> anyhow::Result<Self> {
        // Parse as generic JSON value
        let existing_value: serde_json::Value = serde_json::from_str(content)?;

        // Get default config as JSON value
        let default_config = Config::default();
        let default_value = serde_json::to_value(&default_config)?;

        // Merge existing config with defaults
        let merged_value = Self::merge_json_values(default_value, existing_value);

        // Deserialize merged value back to Config
        let config: Config = serde_json::from_value(merged_value)?;

        // Save the updated config with any missing defaults
        config.save(config_path)?;

        Ok(config)
    }

    fn merge_json_values(
        mut base: serde_json::Value,
        overlay: serde_json::Value,
    ) -> serde_json::Value {
        match (&mut base, overlay) {
            (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
                for (key, value) in overlay_map {
                    base_map
                        .entry(key)
                        .and_modify(|base_value| {
                            *base_value =
                                Self::merge_json_values(base_value.clone(), value.clone());
                        })
                        .or_insert(value);
                }
                base
            }
            (_, overlay) => overlay, // Use overlay value for non-objects
        }
    }

    /// Create a backup of the corrupted config file
    fn backup_corrupted_config(config_path: &PathBuf) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("config_backup_{}.json", timestamp);

        let backup_path = config_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(backup_filename);

        std::fs::copy(config_path, &backup_path)?;
        tracing::info!("Corrupted config backed up to: {}", backup_path.display());
        Ok(())
    }

    pub fn save(&self, config_path: &PathBuf) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
