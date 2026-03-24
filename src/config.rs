use std::env;

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
    pub fn from_env() -> Self {
        let md2wechat_base_url = env::var("MD2WECHAT_BASE_URL")
            .unwrap_or_else(|_| "https://www.md2wechat.cn".to_string());
        let md2wechat_api_key = env::var("MD2WECHAT_API_KEY").ok();

        let appid = env::var("WECHAT_APPID").ok();
        let secret = env::var("WECHAT_SECRET").ok();
        let wechat = match (appid, secret) {
            (Some(appid), Some(secret)) if !appid.is_empty() && !secret.is_empty() => {
                Some(WechatConfig { appid, secret })
            }
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
}
