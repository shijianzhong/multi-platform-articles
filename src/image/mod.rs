use crate::config::Config;
use async_trait::async_trait;
use std::env;

pub mod providers;

#[derive(Debug, Clone)]
pub enum GeneratedImage {
    Url(String),
    Bytes { data: Vec<u8>, ext: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("[{provider}] {message}")]
    Provider { provider: String, message: String },
}

impl ImageError {
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError>;
}

pub fn provider_from_config(cfg: &Config) -> Result<Box<dyn ImageProvider>, ImageError> {
    let provider = cfg
        .image
        .provider
        .clone()
        .or_else(|| env::var("IMAGE_PROVIDER").ok())
        .unwrap_or_else(|| "openai".to_string());
    let provider = provider.trim().to_lowercase();

    let api_key = cfg
        .image
        .api_key
        .clone()
        .or_else(|| env::var("IMAGE_API_KEY").ok())
        .unwrap_or_default();

    let api_base = cfg
        .image
        .api_base
        .clone()
        .or_else(|| env::var("IMAGE_API_BASE").ok())
        .filter(|v| !v.trim().is_empty());
    let model = cfg
        .image
        .model
        .clone()
        .or_else(|| env::var("IMAGE_MODEL").ok())
        .filter(|v| !v.trim().is_empty());
    let size = cfg
        .image
        .size
        .clone()
        .or_else(|| env::var("IMAGE_SIZE").ok())
        .filter(|v| !v.trim().is_empty());

    providers::build_provider(&provider, api_key, api_base, model, size)
}
