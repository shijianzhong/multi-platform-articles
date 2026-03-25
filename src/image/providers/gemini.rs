use crate::image::{GeneratedImage, ImageError, ImageProvider};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
    base_url: String,
    model: String,
    aspect_ratio: String,
    image_size: String,
    http: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        model: Option<String>,
        size: Option<String>,
    ) -> Result<Self, ImageError> {
        if api_key.trim().is_empty() {
            return Err(ImageError::provider(
                "Gemini",
                "IMAGE_API_KEY 未配置（或未在 TUI 中保存）",
            ));
        }

        let base_url = api_base.unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
        let model = model.unwrap_or_else(|| "gemini-3.1-flash-image-preview".to_string());
        let (aspect_ratio, image_size) = map_size_to_gemini(size.as_deref().unwrap_or(""));

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ImageError::provider("Gemini", format!("http client build: {e}")))?;

        Ok(Self {
            api_key,
            base_url,
            model,
            aspect_ratio,
            image_size,
            http,
        })
    }

    fn mime_to_ext(&self, mime: &str) -> String {
        if mime.contains("jpeg") || mime.contains("jpg") {
            ".jpg".to_string()
        } else if mime.contains("gif") {
            ".gif".to_string()
        } else if mime.contains("webp") {
            ".webp".to_string()
        } else {
            ".png".to_string()
        }
    }
}

#[async_trait]
impl ImageProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Gemini"
    }

    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError> {
        #[derive(serde::Serialize)]
        struct Part<'a> {
            text: &'a str,
        }

        #[derive(serde::Serialize)]
        struct Content<'a> {
            role: &'a str,
            parts: Vec<Part<'a>>,
        }

        #[derive(serde::Serialize)]
        struct ImageConfig<'a> {
            #[serde(rename = "aspectRatio")]
            aspect_ratio: &'a str,
            #[serde(rename = "imageSize")]
            image_size: &'a str,
        }

        #[derive(serde::Serialize)]
        struct GenerationConfig<'a> {
            #[serde(rename = "responseModalities")]
            response_modalities: Vec<&'a str>,
            #[serde(rename = "imageConfig")]
            image_config: ImageConfig<'a>,
        }

        #[derive(serde::Serialize)]
        struct Req<'a> {
            contents: Vec<Content<'a>>,
            #[serde(rename = "generationConfig")]
            generation_config: GenerationConfig<'a>,
        }

        #[derive(Debug, Deserialize)]
        struct Resp {
            candidates: Option<Vec<Candidate>>,
            error: Option<ApiError>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiError {
            message: Option<String>,
            status: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct Candidate {
            content: Option<RespContent>,
        }

        #[derive(Debug, Deserialize)]
        struct RespContent {
            parts: Option<Vec<RespPart>>,
        }

        #[derive(Debug, Deserialize)]
        struct RespPart {
            #[serde(rename = "inlineData")]
            inline_data: Option<InlineData>,
            #[serde(rename = "inline_data")]
            inline_data_snake: Option<InlineData>,
        }

        #[derive(Debug, Deserialize)]
        struct InlineData {
            #[serde(rename = "mimeType")]
            mime_type: Option<String>,
            data: String,
        }

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.model,
            self.api_key
        );

        let resp = self
            .http
            .post(url)
            .json(&Req {
                contents: vec![Content {
                    role: "user",
                    parts: vec![Part { text: prompt }],
                }],
                generation_config: GenerationConfig {
                    response_modalities: vec!["TEXT", "IMAGE"],
                    image_config: ImageConfig {
                        aspect_ratio: &self.aspect_ratio,
                        image_size: &self.image_size,
                    },
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
                    .or(err.status)
                    .unwrap_or_else(|| "unknown api error".to_string()),
            ));
        }

        let candidates = parsed
            .candidates
            .ok_or_else(|| ImageError::provider(self.name(), "未收到 candidates"))?;
        let parts = candidates
            .get(0)
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.as_ref())
            .ok_or_else(|| ImageError::provider(self.name(), "响应中没有内容"))?;

        for part in parts {
            let inline = part
                .inline_data
                .as_ref()
                .or(part.inline_data_snake.as_ref());
            let Some(inline) = inline else {
                continue;
            };
            let data = base64::engine::general_purpose::STANDARD
                .decode(inline.data.as_bytes())
                .map_err(|e| ImageError::provider(self.name(), format!("base64 decode: {e}")))?;
            let ext = inline
                .mime_type
                .as_deref()
                .map(|m| self.mime_to_ext(m))
                .unwrap_or_else(|| ".png".to_string());
            return Ok(GeneratedImage::Bytes { data, ext });
        }

        Err(ImageError::provider(self.name(), "响应中没有图片"))
    }
}

fn map_size_to_gemini(size: &str) -> (String, String) {
    if size.trim().is_empty() {
        return ("1:1".to_string(), "1K".to_string());
    }

    let valid_ratios = [
        "1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3", "4:5", "5:4", "21:9",
    ];
    if valid_ratios.iter().any(|r| r == &size) {
        return (size.to_string(), "1K".to_string());
    }

    let mut size_map: HashMap<&'static str, (&'static str, &'static str)> = HashMap::new();
    size_map.insert("1024x1024", ("1:1", "1K"));
    size_map.insert("2048x2048", ("1:1", "2K"));
    size_map.insert("4096x4096", ("1:1", "4K"));
    size_map.insert("848x1264", ("2:3", "1K"));
    size_map.insert("1696x2528", ("2:3", "2K"));
    size_map.insert("3392x5056", ("2:3", "4K"));
    size_map.insert("1264x848", ("3:2", "1K"));
    size_map.insert("2528x1696", ("3:2", "2K"));
    size_map.insert("5056x3392", ("3:2", "4K"));
    size_map.insert("896x1200", ("3:4", "1K"));
    size_map.insert("1792x2400", ("3:4", "2K"));
    size_map.insert("3584x4800", ("3:4", "4K"));
    size_map.insert("1200x896", ("4:3", "1K"));
    size_map.insert("2400x1792", ("4:3", "2K"));
    size_map.insert("4800x3584", ("4:3", "4K"));
    size_map.insert("928x1152", ("4:5", "1K"));
    size_map.insert("1856x2304", ("4:5", "2K"));
    size_map.insert("3712x4608", ("4:5", "4K"));
    size_map.insert("1152x928", ("5:4", "1K"));
    size_map.insert("2304x1856", ("5:4", "2K"));
    size_map.insert("4608x3712", ("5:4", "4K"));
    size_map.insert("768x1376", ("9:16", "1K"));
    size_map.insert("1536x2752", ("9:16", "2K"));
    size_map.insert("3072x5504", ("9:16", "4K"));
    size_map.insert("1376x768", ("16:9", "1K"));
    size_map.insert("2752x1536", ("16:9", "2K"));
    size_map.insert("5504x3072", ("16:9", "4K"));
    size_map.insert("1584x672", ("21:9", "1K"));
    size_map.insert("3168x1344", ("21:9", "2K"));
    size_map.insert("6336x2688", ("21:9", "4K"));

    if let Some((ratio, img_size)) = size_map.get(size) {
        return (ratio.to_string(), img_size.to_string());
    }

    ("1:1".to_string(), "1K".to_string())
}

