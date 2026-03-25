use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub wechat: Option<WechatConfig>,
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ApiConfig {
    pub md2wechat_api_key: Option<String>,
    pub md2wechat_base_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct WechatConfig {
    pub appid: String,
    pub secret: String,
}

impl Config {
    pub fn load() -> Self {
        let file_cfg = FileConfig::load_from_disk().unwrap_or_default();

        let md2wechat_base_url = env::var("MD2WECHAT_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.md2wechat_base_url.clone())
            .unwrap_or_else(|| "https://www.md2wechat.cn".to_string());
        let md2wechat_api_key = env::var("MD2WECHAT_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.md2wechat_api_key.clone());

        let wechat_appid = env::var("WECHAT_APPID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.wechat_appid.clone());
        let wechat_secret = env::var("WECHAT_SECRET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.wechat_secret.clone());
        let wechat = match (wechat_appid, wechat_secret) {
            (Some(appid), Some(secret)) => Some(WechatConfig { appid, secret }),
            _ => None,
        };

        Self {
            wechat,
            api: ApiConfig {
                md2wechat_api_key,
                md2wechat_base_url,
            },
        }
    }

    pub fn save_credentials(
        &self,
        appid: String,
        secret: String,
        md2wechat_api_key: String,
    ) -> Result<(), ConfigError> {
        let mut file_cfg = FileConfig::load_from_disk().unwrap_or_default();
        file_cfg.wechat_appid = Some(appid);
        file_cfg.wechat_secret = Some(secret);
        file_cfg.md2wechat_api_key = Some(md2wechat_api_key);
        file_cfg.save_to_disk()
    }

    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(base.join("mpa").join("config.json"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot resolve user config directory")]
    NoConfigDir,
    #[error("read config file: {0}")]
    Read(#[from] std::io::Error),
    #[error("parse config json: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FileConfig {
    wechat_appid: Option<String>,
    wechat_secret: Option<String>,
    md2wechat_api_key: Option<String>,
    md2wechat_base_url: Option<String>,
}

impl FileConfig {
    fn load_from_disk() -> Result<Self, ConfigError> {
        let path = Config::config_path()?;
        let raw = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn save_to_disk(&self) -> Result<(), ConfigError> {
        let path = Config::config_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }
}
