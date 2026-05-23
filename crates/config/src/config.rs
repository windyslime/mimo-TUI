use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub provider: ProviderKind,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub sandbox_mode: SandboxMode,
    pub approval_policy: ApprovalPolicy,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub muted: String,
    pub surface: String,
    pub text_primary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Mimo,
    Deepseek,
    Openai,
    Openrouter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxMode {
    Off,
    On,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalPolicy {
    Ask,
    AutoApprove,
    Never,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: "cyan".to_string(),
            success: "green".to_string(),
            warning: "yellow".to_string(),
            error: "red".to_string(),
            muted: "darkgray".to_string(),
            surface: "black".to_string(),
            text_primary: "white".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mimo,
            model: "mimo-v2.5-pro".to_string(),
            api_key: None,
            base_url: None,
            sandbox_mode: SandboxMode::Off,
            approval_policy: ApprovalPolicy::Ask,
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> anyhow::Result<PathBuf> {
        Ok(dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mimo")
            .join("config.toml"))
    }

    pub fn base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| match self.provider {
                ProviderKind::Mimo => "https://api.xiaomimimo.com/v1".to_string(),
                ProviderKind::Deepseek => "https://api.deepseek.com/v1".to_string(),
                ProviderKind::Openai => "https://api.openai.com/v1".to_string(),
                ProviderKind::Openrouter => "https://openrouter.ai/api/v1".to_string(),
            })
    }
}
