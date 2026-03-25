use super::{
    DraftArticle, DraftResult, ImagePost, ImagePostResult, PublishError, Publisher, UploadedAsset,
};
use crate::config::WechatConfig;
use async_trait::async_trait;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::path::Path;
use tokio::sync::Mutex;
use url::form_urlencoded;

#[derive(Debug, Clone)]
pub struct WechatPublisher {
    cfg: WechatConfig,
    http: reqwest::Client,
    token_cache: Arc<Mutex<Option<CachedToken>>>,
}

impl WechatPublisher {
    pub fn new(cfg: WechatConfig) -> Result<Self, PublishError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            cfg,
            http,
            token_cache: Arc::new(Mutex::new(None)),
        })
    }

    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    async fn access_token(&self) -> Result<String, PublishError> {
        {
            let guard = self.token_cache.lock().await;
            if let Some(cached) = guard.as_ref() {
                if cached.expires_at > Instant::now() + Duration::from_secs(60) {
                    return Ok(cached.token.clone());
                }
            }
        }

        let appid = form_urlencoded::byte_serialize(self.cfg.appid.as_bytes()).collect::<String>();
        let secret =
            form_urlencoded::byte_serialize(self.cfg.secret.as_bytes()).collect::<String>();
        let url = format!(
            "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
            appid, secret
        );
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if !status.is_success() {
            return Err(PublishError::Message(format!(
                "wechat token http status={} body={}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }
        let parsed: AccessTokenResponse = serde_json::from_slice(&body).map_err(|err| {
            PublishError::Message(format!(
                "parse wechat token response: {err} body={}",
                String::from_utf8_lossy(&body)
            ))
        })?;
        if let Some(errcode) = parsed.errcode {
            if errcode != 0 {
                return Err(PublishError::Message(format!(
                    "wechat token error: {} - {}",
                    errcode,
                    parsed.errmsg.unwrap_or_default()
                )));
            }
        }
        let token = parsed
            .access_token
            .ok_or_else(|| PublishError::Message("missing access_token".to_string()))?;

        let expires_in = parsed.expires_in.unwrap_or(7200);
        let mut guard = self.token_cache.lock().await;
        *guard = Some(CachedToken {
            token: token.clone(),
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        });
        Ok(token)
    }

    async fn create_draft_raw<T: Serialize>(&self, payload: &T) -> Result<String, PublishError> {
        let token = self.access_token().await?;
        let url = format!("https://api.weixin.qq.com/cgi-bin/draft/add?access_token={token}");
        
        // println!("Draft payload: {}", serde_json::to_string_pretty(payload).unwrap_or_default());
        
        let resp = self.http.post(url).json(payload).send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if !status.is_success() {
            return Err(PublishError::Message(format!(
                "wechat draft http status={} body={}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }
        let parsed: DraftAddResponse = serde_json::from_slice(&body).map_err(|err| {
            PublishError::Message(format!(
                "parse wechat draft response: {err} body={}",
                String::from_utf8_lossy(&body)
            ))
        })?;
        if parsed.errcode != 0 {
            return Err(PublishError::Message(format!(
                "wechat api error: {} - {}",
                parsed.errcode, parsed.errmsg
            )));
        }
        Ok(parsed.media_id)
    }

    pub async fn upload_article_image_file(&self, path: &Path) -> Result<String, PublishError> {
        let token = self.access_token().await?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image.png")
            .to_string();
        let bytes = std::fs::read(path)?;

        let url = format!("https://api.weixin.qq.com/cgi-bin/media/uploadimg?access_token={token}");
        let part = multipart::Part::bytes(bytes).file_name(file_name);
        let form = multipart::Form::new().part("media", part);
        let resp = self.http.post(url).multipart(form).send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if !status.is_success() {
            return Err(PublishError::Message(format!(
                "wechat uploadimg http status={} body={}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let parsed: UploadImgResponse = serde_json::from_slice(&body).map_err(|err| {
            PublishError::Message(format!(
                "parse wechat uploadimg response: {err} body={}",
                String::from_utf8_lossy(&body)
            ))
        })?;
        if let Some(errcode) = parsed.errcode {
            if errcode != 0 {
                return Err(PublishError::Message(format!(
                    "wechat api error: {} - {}",
                    errcode,
                    parsed.errmsg.unwrap_or_default()
                )));
            }
        }

        let url = parsed
            .url
            .filter(|u| !u.trim().is_empty())
            .ok_or_else(|| PublishError::Message("missing wechat url from uploadimg".to_string()))?;
        Ok(url)
    }
}

#[async_trait]
impl Publisher for WechatPublisher {
    async fn upload_image_file(&self, path: &Path) -> Result<UploadedAsset, PublishError> {
        let token = self.access_token().await?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image.png")
            .to_string();
        let bytes = std::fs::read(path)?;

        let url = format!(
            "https://api.weixin.qq.com/cgi-bin/material/add_material?access_token={token}&type=image"
        );

        let part = multipart::Part::bytes(bytes).file_name(file_name);
        let form = multipart::Form::new().part("media", part);
        let resp = self.http.post(url).multipart(form).send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if !status.is_success() {
            return Err(PublishError::Message(format!(
                "wechat upload http status={} body={}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let parsed: UploadResponse = serde_json::from_slice(&body).map_err(|err| {
            PublishError::Message(format!(
                "parse wechat upload response: {err} body={}",
                String::from_utf8_lossy(&body)
            ))
        })?;
        if let Some(errcode) = parsed.errcode {
            if errcode != 0 {
                return Err(PublishError::Message(format!(
                    "wechat api error: {} - {}",
                    errcode,
                    parsed.errmsg.unwrap_or_default()
                )));
            }
        }

        let media_id = parsed
            .media_id
            .ok_or_else(|| PublishError::Message("missing media_id".to_string()))?;

        Ok(UploadedAsset {
            media_id,
            url: parsed.url,
        })
    }

    async fn create_draft(&self, articles: Vec<DraftArticle>) -> Result<DraftResult, PublishError> {
        let payload = DraftAddRequest {
            articles: articles
                .into_iter()
                .map(|a| DraftArticlePayload {
                    title: a.title,
                    author: a.author,
                    digest: a.digest,
                    content: a.content_html,
                    content_source_url: a.content_source_url,
                    thumb_media_id: a.cover_media_id.clone(),
                    show_cover_pic: Some(if a.show_cover_pic && a.cover_media_id.is_some() { 1 } else { 0 }),
                })
                .collect(),
        };
        let media_id = self.create_draft_raw(&payload).await?;
        Ok(DraftResult {
            media_id,
            draft_url: None,
        })
    }

    async fn create_image_post(&self, post: ImagePost) -> Result<ImagePostResult, PublishError> {
        let image_list = post
            .image_media_ids
            .into_iter()
            .map(|id| NewspicImageItem { image_media_id: id })
            .collect();

        let mut article = NewspicArticlePayload {
            title: post.title,
            content: post.content,
            article_type: "newspic".to_string(),
            image_info: NewspicImageInfo { image_list },
            need_open_comment: None,
            only_fans_can_comment: None,
        };
        if post.open_comment {
            article.need_open_comment = Some(1);
            if post.fans_only {
                article.only_fans_can_comment = Some(1);
            }
        }

        let payload = NewspicDraftAddRequest {
            articles: vec![article],
        };
        let media_id = self.create_draft_raw(&payload).await?;
        Ok(ImagePostResult {
            media_id,
            draft_url: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: Instant,
}

#[derive(Debug, Serialize)]
struct DraftAddRequest {
    articles: Vec<DraftArticlePayload>,
}

#[derive(Debug, Serialize)]
struct DraftArticlePayload {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb_media_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_cover_pic: Option<u32>, // 微信 API 对此字段的要求可能比较严格
}

#[derive(Debug, Serialize)]
struct NewspicDraftAddRequest {
    articles: Vec<NewspicArticlePayload>,
}

#[derive(Debug, Serialize)]
struct NewspicArticlePayload {
    title: String,
    content: String,
    article_type: String,
    image_info: NewspicImageInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    need_open_comment: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    only_fans_can_comment: Option<i32>,
}

#[derive(Debug, Serialize)]
struct NewspicImageInfo {
    image_list: Vec<NewspicImageItem>,
}

#[derive(Debug, Serialize)]
struct NewspicImageItem {
    image_media_id: String,
}

#[derive(Debug, Deserialize)]
struct DraftAddResponse {
    errcode: i64,
    errmsg: String,
    media_id: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    media_id: Option<String>,
    url: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadImgResponse {
    url: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}
