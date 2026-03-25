use crate::image::{GeneratedImage, ImageError, ImageProvider};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    api_key: String,
    base_url: String,
    model: String,
    aspect_ratio: String,
    image_size: String,
    http: reqwest::Client,
}

impl OpenRouterProvider {
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        model: Option<String>,
        size: Option<String>,
    ) -> Result<Self, ImageError> {
        if api_key.trim().is_empty() {
            return Err(ImageError::provider(
                "OpenRouter",
                "IMAGE_API_KEY 未配置（或未在 TUI 中保存）",
            ));
        }

        let base_url = api_base.unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
        let model = model.unwrap_or_else(|| "google/gemini-3-pro-image-preview".to_string());

        let (aspect_ratio, image_size) = map_size_to_openrouter(size.as_deref().unwrap_or(""));

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ImageError::provider("OpenRouter", format!("http client build: {e}")))?;

        Ok(Self {
            api_key,
            base_url,
            model,
            aspect_ratio,
            image_size,
            http,
        })
    }

    fn parse_data_url(&self, data_url: &str) -> Result<(Vec<u8>, String), ImageError> {
        let Some(rest) = data_url.strip_prefix("data:") else {
            return Err(ImageError::provider(self.name(), "invalid data url"));
        };
        let Some((meta, b64)) = rest.split_once(',') else {
            return Err(ImageError::provider(self.name(), "invalid data url"));
        };
        let ext = if meta.contains("image/jpeg") || meta.contains("image/jpg") {
            ".jpg"
        } else if meta.contains("image/gif") {
            ".gif"
        } else if meta.contains("image/webp") {
            ".webp"
        } else {
            ".png"
        };
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| ImageError::provider(self.name(), format!("base64 decode: {e}")))?;
        Ok((data, ext.to_string()))
    }
}

#[async_trait]
impl ImageProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        "OpenRouter"
    }

    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError> {
        #[derive(serde::Serialize)]
        struct Message<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            modalities: Vec<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            image_config: Option<HashMap<&'a str, String>>,
        }

        #[derive(Debug, Deserialize)]
        struct Resp {
            choices: Vec<Choice>,
            error: Option<RespError>,
        }

        #[derive(Debug, Deserialize)]
        struct RespError {
            message: Option<String>,
            r#type: Option<String>,
            code: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct Choice {
            message: RespMessage,
        }

        #[derive(Debug, Deserialize)]
        struct RespMessage {
            images: Option<Vec<ImageItem>>,
        }

        #[derive(Debug, Deserialize)]
        struct ImageItem {
            image_url: ImageUrl,
        }

        #[derive(Debug, Deserialize)]
        struct ImageUrl {
            url: String,
        }

        let mut image_config = HashMap::new();
        if !self.aspect_ratio.trim().is_empty() {
            image_config.insert("aspect_ratio", self.aspect_ratio.clone());
        }
        if !self.image_size.trim().is_empty() {
            image_config.insert("image_size", self.image_size.clone());
        }

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://md2wechat.cn")
            .header("X-Title", "md2wechat")
            .json(&Req {
                model: &self.model,
                messages: vec![Message {
                    role: "user",
                    content: prompt,
                }],
                modalities: vec!["image"],
                image_config: if image_config.is_empty() {
                    None
                } else {
                    Some(image_config)
                },
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

        if let Some(err) = parsed.error {
            return Err(ImageError::provider(
                self.name(),
                err.message
                    .or(err.code)
                    .or(err.r#type)
                    .unwrap_or_else(|| "unknown api error".to_string()),
            ));
        }

        let data_url = parsed
            .choices
            .get(0)
            .and_then(|c| c.message.images.as_ref())
            .and_then(|imgs| imgs.get(0))
            .map(|i| i.image_url.url.as_str())
            .ok_or_else(|| ImageError::provider(self.name(), "未生成图片"))?;

        let (data, ext) = self.parse_data_url(data_url)?;
        Ok(GeneratedImage::Bytes { data, ext })
    }
}

fn map_size_to_openrouter(size: &str) -> (String, String) {
    if size.trim().is_empty() {
        return ("1:1".to_string(), "2K".to_string());
    }

    let mut size_map: HashMap<&'static str, (&'static str, &'static str)> = HashMap::new();
    size_map.insert("1024x1024", ("1:1", "1K"));
    size_map.insert("2048x2048", ("1:1", "2K"));
    size_map.insert("4096x4096", ("1:1", "4K"));
    size_map.insert("1344x768", ("16:9", "1K"));
    size_map.insert("1920x1080", ("16:9", "2K"));
    size_map.insert("2560x1440", ("16:9", "2K"));
    size_map.insert("3840x2160", ("16:9", "4K"));
    size_map.insert("768x1344", ("9:16", "1K"));
    size_map.insert("1080x1920", ("9:16", "2K"));
    size_map.insert("1440x2560", ("9:16", "2K"));
    size_map.insert("2160x3840", ("9:16", "4K"));
    size_map.insert("1184x864", ("4:3", "1K"));
    size_map.insert("1600x1200", ("4:3", "2K"));
    size_map.insert("2048x1536", ("4:3", "2K"));
    size_map.insert("864x1184", ("3:4", "1K"));
    size_map.insert("1200x1600", ("3:4", "2K"));
    size_map.insert("1536x2048", ("3:4", "2K"));
    size_map.insert("1248x832", ("3:2", "1K"));
    size_map.insert("1800x1200", ("3:2", "2K"));
    size_map.insert("3072x2048", ("3:2", "4K"));
    size_map.insert("832x1248", ("2:3", "1K"));
    size_map.insert("1200x1800", ("2:3", "2K"));
    size_map.insert("2048x3072", ("2:3", "4K"));
    size_map.insert("1152x896", ("5:4", "1K"));
    size_map.insert("896x1152", ("4:5", "1K"));
    size_map.insert("1536x672", ("21:9", "1K"));

    if let Some((ratio, img_size)) = size_map.get(size) {
        return (ratio.to_string(), img_size.to_string());
    }

    let valid_ratios = [
        "1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9",
    ];
    if valid_ratios.iter().any(|r| r == &size) {
        return (size.to_string(), "2K".to_string());
    }

    ("1:1".to_string(), "2K".to_string())
}

