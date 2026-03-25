use crate::image::{provider_from_config, GeneratedImage};
use crate::platforms::wechat::WechatPublisher;
use crate::publish::{AssetError, AssetProcessor, UploadResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Clone)]
pub struct WechatAssetProcessor {
    cfg: crate::config::Config,
    publisher: WechatPublisher,
    http: reqwest::Client,
}

impl WechatAssetProcessor {
    pub fn new(cfg: crate::config::Config, publisher: WechatPublisher) -> Result<Self, AssetError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| AssetError::Message(format!("build http client: {e}")))?;
        Ok(Self {
            cfg,
            publisher,
            http,
        })
    }

    fn temp_file_path(&self, ext: &str) -> Result<PathBuf, AssetError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AssetError::Message(format!("time error: {e}")))?;
        let nanos = now.as_nanos();
        let pid = std::process::id();
        let mut name = format!("mpa-img-{pid}-{nanos}");
        let ext = ext.trim();
        if !ext.is_empty() {
            if ext.starts_with('.') {
                name.push_str(ext);
            } else {
                name.push('.');
                name.push_str(ext);
            }
        } else {
            name.push_str(".png");
        }
        Ok(std::env::temp_dir().join(name))
    }

    fn ext_from_url(&self, url: &str) -> String {
        let url = url.split('?').next().unwrap_or(url);
        let last = url.rsplit('/').next().unwrap_or("");
        if let Some((_, ext)) = last.rsplit_once('.') {
            let ext = ext.trim().to_lowercase();
            if ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                return format!(".{ext}");
            }
        }
        ".png".to_string()
    }

    async fn upload_path(&self, path: &Path) -> Result<UploadResult, AssetError> {
        let url = self
            .publisher
            .upload_article_image_file(path)
            .await
            .map_err(|e| AssetError::Message(e.to_string()))?;
        if url.trim().is_empty() {
            return Err(AssetError::Message(
                "wechat uploadimg returned empty url".to_string(),
            ));
        }
        Ok(UploadResult {
            media_id: String::new(),
            wechat_url: url,
        })
    }

    async fn download_to_temp(&self, url: &str) -> Result<PathBuf, AssetError> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AssetError::Message(format!("download failed: {e}")))?;
        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| AssetError::Message(format!("read download body: {e}")))?;
        if !status.is_success() {
            return Err(AssetError::Message(format!(
                "download http status={status} body={}",
                String::from_utf8_lossy(&body)
            )));
        }
        let ext = self.ext_from_url(url);
        let path = self.temp_file_path(&ext)?;
        std::fs::write(&path, &body)
            .map_err(|e| AssetError::Message(format!("write temp file: {e}")))?;
        Ok(path)
    }
}

#[async_trait]
impl AssetProcessor for WechatAssetProcessor {
    async fn upload_local_image(&self, file_path: &str) -> Result<UploadResult, AssetError> {
        self.upload_path(Path::new(file_path)).await
    }

    async fn download_and_upload(&self, url: &str) -> Result<UploadResult, AssetError> {
        let tmp = self.download_to_temp(url).await?;
        let result = self.upload_path(&tmp).await;
        let _ = std::fs::remove_file(&tmp);
        result
    }

    async fn generate_and_upload(&self, prompt: &str) -> Result<UploadResult, AssetError> {
        let provider = provider_from_config(&self.cfg)
            .map_err(|e| AssetError::Message(e.to_string()))?;
        match provider
            .generate(prompt)
            .await
            .map_err(|e| AssetError::Message(e.to_string()))?
        {
            GeneratedImage::Url(url) => self.download_and_upload(&url).await,
            GeneratedImage::Bytes { data, ext } => {
                let tmp = self.temp_file_path(&ext)?;
                std::fs::write(&tmp, &data)
                    .map_err(|e| AssetError::Message(format!("write temp file: {e}")))?;
                let result = self.upload_path(&tmp).await;
                let _ = std::fs::remove_file(&tmp);
                result
            }
        }
    }
}
