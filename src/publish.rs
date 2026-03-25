use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Local,
    Remote,
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRef {
    pub index: usize,
    pub kind: AssetKind,
    pub source: String,
    pub resolved_source: Option<String>,
    pub prompt: Option<String>,
    pub placeholder: Option<String>,
    pub media_id: Option<String>,
    pub public_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub media_id: String,
    pub wechat_url: String,
}

#[async_trait]
pub trait AssetProcessor: Send + Sync {
    async fn upload_local_image(&self, file_path: &str) -> Result<UploadResult, AssetError>;
    async fn download_and_upload(&self, url: &str) -> Result<UploadResult, AssetError>;
    async fn generate_and_upload(&self, prompt: &str) -> Result<UploadResult, AssetError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone)]
pub struct AssetPipeline<P> {
    processor: P,
}

impl<P> AssetPipeline<P>
where
    P: AssetProcessor,
{
    pub fn new(processor: P) -> Self {
        Self { processor }
    }

    pub async fn process(&self, input: &ProcessInput) -> Result<ProcessOutput, AssetError> {
        if input.assets.is_empty() {
            return Ok(ProcessOutput {
                html: input.html.clone(),
                assets: input.assets.clone(),
            });
        }

        let mut output = ProcessOutput {
            html: insert_asset_placeholders(&input.html, &input.assets),
            assets: input.assets.clone(),
        };

        let mut failures = Vec::new();
        for (i, asset) in output.assets.clone().into_iter().enumerate() {
            let result = match asset.kind {
                AssetKind::Local => {
                    let path = asset
                        .resolved_source
                        .as_deref()
                        .unwrap_or(asset.source.as_str());
                    self.processor.upload_local_image(path).await
                }
                AssetKind::Remote => self.processor.download_and_upload(&asset.source).await,
                AssetKind::Ai => {
                    let prompt = asset.prompt.as_deref().unwrap_or(asset.source.as_str());
                    self.processor.generate_and_upload(prompt).await
                }
            };

            match result {
                Ok(upload) => {
                    if upload.wechat_url.trim().is_empty() {
                        failures.push(format!("{i}:empty wechat url"));
                        continue;
                    }
                    output.assets[i].media_id = Some(upload.media_id);
                    output.assets[i].public_url = Some(upload.wechat_url);
                }
                Err(err) => {
                    failures.push(format!("{i}:{err}"));
                }
            }
        }

        output.html = replace_asset_placeholders(&output.html, &output.assets);

        if !failures.is_empty() {
            return Err(AssetError::Message(format!(
                "asset processing failed for {} asset(s): {}",
                failures.len(),
                failures.join("; ")
            )));
        }

        Ok(output)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInput {
    pub html: String,
    pub assets: Vec<AssetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutput {
    pub html: String,
    pub assets: Vec<AssetRef>,
}

pub fn insert_asset_placeholders(html: &str, assets: &[AssetRef]) -> String {
    let mut result = html.to_string();
    let max_index = assets.iter().map(|a| a.index).max().unwrap_or(0);
    let mut inserted = vec![false; max_index.saturating_add(1)];

    for asset in assets {
        let Some(placeholder) = asset.placeholder.as_deref() else {
            continue;
        };
        if placeholder.is_empty() {
            continue;
        }
        let source = &asset.source;
        if source.is_empty() {
            continue;
        }

        let quoted = regex::escape(source);
        let double = Regex::new(&format!(r#"(?i)<img[^>]*src="{quoted}"[^>]*>"#)).ok();
        let single = Regex::new(&format!(r#"(?i)<img[^>]*src='{quoted}'[^>]*>"#)).ok();

        if let Some(re) = &double {
            if re.is_match(&result) {
                if let Some(slot) = inserted.get_mut(asset.index) {
                    *slot = true;
                }
            }
            result = re.replace_all(&result, placeholder).into_owned();
        }
        if let Some(re) = &single {
            if re.is_match(&result) {
                if let Some(slot) = inserted.get_mut(asset.index) {
                    *slot = true;
                }
            }
            result = re.replace_all(&result, placeholder).into_owned();
        }
    }

    let img_tag = Regex::new(r"(?i)<img\b[^>]*>").expect("img tag regex");
    for asset in assets {
        let Some(placeholder) = asset.placeholder.as_deref() else {
            continue;
        };
        if inserted.get(asset.index).copied().unwrap_or(false) || placeholder.is_empty() {
            continue;
        }
        let placeholder = placeholder.to_string();
        let mut done = false;
        result = img_tag
            .replace_all(&result, |_: &regex::Captures| {
                if done {
                    return String::new();
                }
                done = true;
                placeholder.clone()
            })
            .into_owned();
    }

    result
}

pub fn replace_asset_placeholders(html: &str, assets: &[AssetRef]) -> String {
    let mut result = html.to_string();
    for asset in assets {
        let Some(url) = asset.public_url.as_deref() else {
            continue;
        };
        if let Some(placeholder) = asset.placeholder.as_deref() {
            if !placeholder.is_empty() {
                let img_tag = format!(
                    r#"<img src="{url}" style="max-width:100%;height:auto;display:block;margin:20px auto;" />"#
                );
                result = result.replace(placeholder, &img_tag);
            }
        }
        result = result.replace(
            &format!(r#"src="{}""#, asset.source),
            &format!(r#"src="{url}""#),
        );
        result = result.replace(
            &format!(r#"src='{}'"#, asset.source),
            &format!(r#"src='{url}'"#),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_replaces_placeholders() {
        let html = r#"<p>a</p><img src="./a.png"><p>b</p><img src="https://example.com/b.png">"#;
        let assets = vec![
            AssetRef {
                index: 0,
                kind: AssetKind::Local,
                source: "./a.png".to_string(),
                resolved_source: None,
                prompt: None,
                placeholder: Some("<!-- IMG:0 -->".to_string()),
                media_id: Some("m-a".to_string()),
                public_url: Some("https://wechat.local/a".to_string()),
            },
            AssetRef {
                index: 1,
                kind: AssetKind::Remote,
                source: "https://example.com/b.png".to_string(),
                resolved_source: None,
                prompt: None,
                placeholder: Some("<!-- IMG:1 -->".to_string()),
                media_id: Some("m-b".to_string()),
                public_url: Some("https://wechat.local/b".to_string()),
            },
        ];

        let with_placeholders = insert_asset_placeholders(html, &assets);
        assert!(with_placeholders.contains("<!-- IMG:0 -->"));
        assert!(with_placeholders.contains("<!-- IMG:1 -->"));

        let replaced = replace_asset_placeholders(&with_placeholders, &assets);
        assert!(replaced.contains("https://wechat.local/a"));
        assert!(replaced.contains("https://wechat.local/b"));
        assert!(!replaced.contains("./a.png"));
    }
}
