use crate::image::{GeneratedImage, ImageError, ImageProvider};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    size: String,
    http: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        model: Option<String>,
        size: Option<String>,
    ) -> Result<Self, ImageError> {
        if api_key.trim().is_empty() {
            return Err(ImageError::provider(
                "OpenAI",
                "IMAGE_API_KEY 未配置（或未在 TUI 中保存）",
            ));
        }

        let base_url = api_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let model = model.unwrap_or_else(|| "gpt-image-1.5".to_string());
        let size = size.unwrap_or_else(|| "1024x1024".to_string());

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ImageError::provider("OpenAI", format!("http client build: {e}")))?;

        Ok(Self {
            api_key,
            base_url,
            model,
            size,
            http,
        })
    }
}

#[async_trait]
impl ImageProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "OpenAI"
    }

    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError> {
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
            n: u32,
            size: &'a str,
        }

        #[derive(Debug, Deserialize)]
        struct RespItem {
            url: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct Resp {
            data: Vec<RespItem>,
            #[serde(default)]
            error: Option<OpenAIError>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenAIError {
            message: Option<String>,
            r#type: Option<String>,
            code: Option<String>,
        }

        let url = format!("{}/images/generations", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&Req {
                model: &self.model,
                prompt,
                n: 1,
                size: &self.size,
            })
            .send()
            .await
            .map_err(|e| ImageError::provider(self.name(), format!("network error: {e}")))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| ImageError::provider(self.name(), format!("read response: {e}")))?;

        if !status.is_success() {
            let msg = String::from_utf8_lossy(&body).to_string();
            return Err(ImageError::provider(
                self.name(),
                format!("http status={status} body={msg}"),
            ));
        }

        let parsed: Resp = serde_json::from_slice(&body).map_err(|e| {
            ImageError::provider(
                self.name(),
                format!("parse response: {e} body={}", String::from_utf8_lossy(&body)),
            )
        })?;

        if let Some(err) = parsed.error {
            return Err(ImageError::provider(
                self.name(),
                err.message
                    .or(err.code)
                    .or(err.r#type)
                    .unwrap_or_else(|| "unknown api error".to_string()),
            ));
        }

        let Some(item) = parsed.data.into_iter().next() else {
            return Err(ImageError::provider(self.name(), "未生成图片"));
        };
        let Some(url) = item.url else {
            return Err(ImageError::provider(self.name(), "响应中缺少图片 URL"));
        };

        Ok(GeneratedImage::Url(url))
    }
}
