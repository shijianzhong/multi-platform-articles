use crate::image::{GeneratedImage, ImageError, ImageProvider};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone)]
pub struct ModelScopeProvider {
    api_key: String,
    base_url: String,
    model: String,
    size: String,
    http: reqwest::Client,
    poll_interval: Duration,
    max_poll_time: Duration,
}

impl ModelScopeProvider {
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        model: Option<String>,
        size: Option<String>,
    ) -> Result<Self, ImageError> {
        if api_key.trim().is_empty() {
            return Err(ImageError::provider(
                "ModelScope",
                "IMAGE_API_KEY 未配置（或未在 TUI 中保存）",
            ));
        }

        let base_url = api_base.unwrap_or_else(|| "https://api-inference.modelscope.cn".to_string());
        let model = model.unwrap_or_else(|| "Tongyi-MAI/Z-Image-Turbo".to_string());
        let size = size.unwrap_or_else(|| "1024x1024".to_string());

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ImageError::provider("ModelScope", format!("http client build: {e}")))?;

        Ok(Self {
            api_key,
            base_url,
            model,
            size,
            http,
            poll_interval: Duration::from_secs(5),
            max_poll_time: Duration::from_secs(120),
        })
    }

    fn parse_size(&self) -> Result<(u32, u32), ImageError> {
        let raw = self.size.trim();
        let mut it = raw.split('x');
        let w = it
            .next()
            .ok_or_else(|| ImageError::provider(self.name(), "图片尺寸格式错误，期望 WIDTHxHEIGHT"))?;
        let h = it
            .next()
            .ok_or_else(|| ImageError::provider(self.name(), "图片尺寸格式错误，期望 WIDTHxHEIGHT"))?;
        if it.next().is_some() {
            return Err(ImageError::provider(
                self.name(),
                "图片尺寸格式错误，期望 WIDTHxHEIGHT",
            ));
        }
        let w: u32 = w.trim().parse().map_err(|_| {
            ImageError::provider(self.name(), "图片尺寸格式错误（宽度不是数字）")
        })?;
        let h: u32 = h.trim().parse().map_err(|_| {
            ImageError::provider(self.name(), "图片尺寸格式错误（高度不是数字）")
        })?;
        Ok((w, h))
    }

    async fn create_task(&self, prompt: &str) -> Result<String, ImageError> {
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
            n: u32,
            width: u32,
            height: u32,
        }

        #[derive(Debug, Deserialize)]
        struct Resp {
            task_id: Option<String>,
        }

        let (width, height) = self.parse_size()?;
        let url = format!(
            "{}/v1/images/generations",
            self.base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .header("X-ModelScope-Async-Mode", "true")
            .json(&Req {
                model: &self.model,
                prompt,
                n: 1,
                width,
                height,
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
        parsed.task_id.ok_or_else(|| ImageError::provider(self.name(), "未获取到 task_id"))
    }

    async fn get_task_status(&self, task_id: &str) -> Result<(String, Option<String>), ImageError> {
        #[derive(Debug, Deserialize)]
        struct Resp {
            task_status: Option<String>,
            output_images: Option<Vec<String>>,
            error_message: Option<String>,
        }

        let url = format!(
            "{}/v1/tasks/{}",
            self.base_url.trim_end_matches('/'),
            task_id
        );
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.api_key)
            .header("X-ModelScope-Task-Type", "image_generation")
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
        if parsed.error_message.as_deref().unwrap_or("").trim().len() > 0 {
            return Ok(("FAILED".to_string(), None));
        }
        let status = parsed
            .task_status
            .unwrap_or_else(|| "PENDING".to_string());
        let url = parsed
            .output_images
            .and_then(|mut v| v.drain(..).next())
            .filter(|s| !s.trim().is_empty());
        Ok((status, url))
    }

    async fn poll_task(&self, task_id: &str) -> Result<String, ImageError> {
        let start = Instant::now();
        loop {
            if start.elapsed() > self.max_poll_time {
                return Err(ImageError::provider(
                    self.name(),
                    format!("图片生成超时（超过 {:?}）", self.max_poll_time),
                ));
            }
            let (status, url) = self.get_task_status(task_id).await?;
            match status.as_str() {
                "SUCCEED" => {
                    return url.ok_or_else(|| {
                        ImageError::provider(self.name(), "任务成功但未返回图片 URL")
                    })
                }
                "FAILED" => {
                    return Err(ImageError::provider(self.name(), "图片生成任务失败"));
                }
                _ => {
                    sleep(self.poll_interval).await;
                }
            }
        }
    }
}

#[async_trait]
impl ImageProvider for ModelScopeProvider {
    fn name(&self) -> &'static str {
        "ModelScope"
    }

    async fn generate(&self, prompt: &str) -> Result<GeneratedImage, ImageError> {
        let task_id = self.create_task(prompt).await?;
        let url = self.poll_task(&task_id).await?;
        Ok(GeneratedImage::Url(url))
    }
}

