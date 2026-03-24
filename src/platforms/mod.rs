pub mod wechat;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedAsset {
    pub media_id: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftArticle {
    pub title: String,
    pub author: Option<String>,
    pub digest: Option<String>,
    pub content_html: String,
    pub cover_media_id: Option<String>,
    pub show_cover_pic: bool,
    pub content_source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftResult {
    pub media_id: String,
    pub draft_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePost {
    pub title: String,
    pub content: String,
    pub image_media_ids: Vec<String>,
    pub open_comment: bool,
    pub fans_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePostResult {
    pub media_id: String,
    pub draft_url: Option<String>,
}

#[async_trait]
pub trait Publisher: Send + Sync {
    async fn upload_image_file(&self, path: &Path) -> Result<UploadedAsset, PublishError>;
    async fn create_draft(&self, articles: Vec<DraftArticle>) -> Result<DraftResult, PublishError>;
    async fn create_image_post(&self, post: ImagePost) -> Result<ImagePostResult, PublishError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("{0}")]
    Message(String),
}
