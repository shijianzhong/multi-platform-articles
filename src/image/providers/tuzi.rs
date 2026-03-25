use crate::image::{GeneratedImage, ImageError, ImageProvider};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TuZiProvider {
    api_key: String,
    base_url: String,
    model: String,
    size: String,
    http: reqwest::Client,
}

impl TuZiProvider {
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        model: Option<String>,
        size: Option<String>,
    ) -> Result<Self, ImageError> {
        if api_key.trim().is_empty() {
            return Err(ImageError::provider(
                "TuZi",
                "IMAGE_API_KEY 未配置（或未在 TUI 中保存）",
            ));
        }
        let Some(base_url) = api_base.filter(|v| !v.trim().is_empty()) else {
            return Err(ImageError::provider(
                "TuZi",
                "TuZi 需要配置 IMAGE_API_BASE（例如 https://api.tu-zi.com/v1）",
            ));
        };

        let model = model.unwrap_or_else(|| "doubao-seedream-4-5-251128".to_string());
        let size = size.unwrap_or_else(|| "2048x2048".to_string());

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ImageError::provider("TuZi", format!("http client build: {e}")))?;

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
impl ImageProvider for TuZiProvider {
    fn name(&self) -> &'static str {
        "TuZi"
    }

    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError> {
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
            n: u32,
            size: &'a str,
            response_format: &'a str,
        }

        #[derive(Debug, Deserialize)]
        struct RespItem {
            url: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct Resp {
            data: Vec<RespItem>,
        }

        let url = format!("{}/images/generations", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://md2wechat.cn")
            .header("X-Title", "WeChat Markdown Editor")
            .json(&Req {
                model: &self.model,
                prompt,
                n: 1,
                size: &self.size,
                response_format: "url",
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
            return Err(ImageError::provider(
                self.name(),
                format!(
                    "http status={status} body={}",
                    String::from_utf8_lossy(&body)
                ),
            ));
        }

        let parsed: Resp = serde_json::from_slice(&body).map_err(|e| {
            ImageError::provider(
                self.name(),
                format!("parse response: {e} body={}", String::from_utf8_lossy(&body)),
            )
        })?;
        let Some(url) = parsed.data.into_iter().next().and_then(|i| i.url) else {
            return Err(ImageError::provider(self.name(), "未生成图片"));
        };
        Ok(GeneratedImage::Url(url))
    }
}

