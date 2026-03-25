use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub wechat: Option<WechatConfig>,
    pub api: ApiConfig,
    pub image: ImageConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ImageConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub model: Option<String>,
    pub size: Option<String>,
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

        let md2wechat_appid = env::var("WECHAT_APPID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.wechat_appid.clone());
        let wechat_secret = env::var("WECHAT_SECRET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.wechat_secret.clone());
        let wechat = match (md2wechat_appid, wechat_secret) {
            (Some(appid), Some(secret)) => Some(WechatConfig { appid, secret }),
            _ => None,
        };

        let image_provider = env::var("IMAGE_PROVIDER")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.image_provider.clone());
        let image_api_key = env::var("IMAGE_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.image_api_key.clone());
        let image_api_base = env::var("IMAGE_API_BASE")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.image_api_base.clone());
        let image_model = env::var("IMAGE_MODEL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.image_model.clone());
        let image_size = env::var("IMAGE_SIZE")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.image_size.clone());

        Self {
            wechat,
            api: ApiConfig {
                md2wechat_api_key,
                md2wechat_base_url,
            },
            image: ImageConfig {
                provider: image_provider,
                api_key: image_api_key,
                api_base: image_api_base,
                model: image_model,
                size: image_size,
            },
        }
    }

    pub fn save_credentials(
        &self,
        appid: String,
        secret: String,
        image_provider: String,
        image_api_key: String,
        image_api_base: String,
        image_model: String,
        image_size: String,
    ) -> Result<(), ConfigError> {
        let mut file_cfg = FileConfig::load_from_disk().unwrap_or_default();
        file_cfg.wechat_appid = Some(appid);
        file_cfg.wechat_secret = Some(secret);
        file_cfg.image_provider = Some(image_provider);
        file_cfg.image_api_key = Some(image_api_key);
        file_cfg.image_api_base = Some(image_api_base);
        file_cfg.image_model = Some(image_model);
        file_cfg.image_size = Some(image_size);
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
    image_provider: Option<String>,
    image_api_key: Option<String>,
    image_api_base: Option<String>,
    image_model: Option<String>,
    image_size: Option<String>,
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
