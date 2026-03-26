use crate::config::ApiConfig;
use crate::theme::{Palette, Theme, ThemeKind, ThemeManager};
use once_cell::sync::Lazy;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvertMode {
    Api,
    Ai,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Completed,
    ActionRequired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    Local,
    Remote,
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRef {
    pub index: usize,
    pub original: String,
    pub placeholder: String,
    pub wechat_url: Option<String>,
    pub kind: ImageKind,
    pub ai_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertRequest {
    pub markdown: String,
    pub mode: ConvertMode,
    pub theme: String,
    pub api_key: Option<String>,
    pub font_size: Option<String>,
    pub background_type: Option<String>,
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResult {
    pub html: Option<String>,
    pub mode: ConvertMode,
    pub theme: String,
    pub images: Vec<ImageRef>,
    pub status: ResultStatus,
    pub retryable: bool,
    pub prompt: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArticleMetadata {
    pub title: String,
    pub author: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("markdown content cannot be empty")]
    EmptyMarkdown,
    #[error("API key is required for api mode")]
    MissingApiKey,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid api response: {0}")]
    InvalidApiResponse(String),
    #[error("theme not found: {0}")]
    ThemeNotFound(String),
}

#[derive(Debug)]
pub struct MarkdownConverter {
    api: ApiConfig,
    themes: ThemeManager,
    http: reqwest::Client,
}

impl MarkdownConverter {
    pub fn new(api: ApiConfig, themes: ThemeManager) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build");
        Self { api, themes, http }
    }

    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    pub fn themes(&self) -> &ThemeManager {
        &self.themes
    }

    pub async fn convert(&self, mut req: ConvertRequest) -> Result<ConvertResult, ConvertError> {
        if req.markdown.trim().is_empty() {
            return Err(ConvertError::EmptyMarkdown);
        }
        if req.theme.trim().is_empty() {
            req.theme = "default".to_string();
        }

        let images = parse_markdown_images(&req.markdown);
        match req.mode {
            ConvertMode::Api => self.convert_via_api(req, images).await,
            ConvertMode::Ai => Ok(self.convert_via_ai(req, images).await),
            ConvertMode::Local => Ok(self.convert_via_local(req, images)),
        }
    }

    async fn convert_via_api(
        &self,
        req: ConvertRequest,
        images: Vec<ImageRef>,
    ) -> Result<ConvertResult, ConvertError> {
        let api_key = req
            .api_key
            .clone()
            .or_else(|| self.api.md2wechat_api_key.clone())
            .ok_or(ConvertError::MissingApiKey)?;

        let api_theme = match self.themes.get(&req.theme) {
            Some(theme) if theme.kind() == ThemeKind::Api => theme
                .api_theme
                .clone()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| req.theme.clone()),
            _ => req.theme.clone(),
        };

        let url = resolve_md2wechat_convert_url(&self.api.md2wechat_base_url);
        let api_req = ApiConvertRequest {
            markdown: req.markdown,
            theme: api_theme,
            font_size: req.font_size,
            background_type: req.background_type,
        };

        let resp = self
            .http
            .post(url)
            .header("X-API-Key", api_key)
            .json(&api_req)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.bytes().await?;
        if !status.is_success() {
            return Err(ConvertError::InvalidApiResponse(format!(
                "status={} body={}",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let api_resp: ApiConvertResponse = serde_json::from_slice(&body).map_err(|err| {
            ConvertError::InvalidApiResponse(format!(
                "parse json: {err} body={}",
                String::from_utf8_lossy(&body)
            ))
        })?;

        if api_resp.code != 0 {
            return Err(ConvertError::InvalidApiResponse(format!(
                "code={} msg={}",
                api_resp.code, api_resp.msg
            )));
        }

        let mut html = api_resp.data.html;
        html = insert_image_placeholders(&html, &images);

        Ok(ConvertResult {
            html: Some(html),
            mode: ConvertMode::Api,
            theme: req.theme,
            images,
            status: ResultStatus::Completed,
            retryable: false,
            prompt: None,
            error: None,
        })
    }

    async fn convert_via_ai(&self, req: ConvertRequest, images: Vec<ImageRef>) -> ConvertResult {
        let prompt = build_ai_prompt(&self.themes, &req.theme, &req.markdown, req.custom_prompt);
        ConvertResult {
            html: None,
            mode: ConvertMode::Ai,
            theme: req.theme,
            images,
            status: ResultStatus::ActionRequired,
            retryable: false,
            prompt: Some(prompt),
            error: None,
        }
    }

    fn convert_via_local(&self, req: ConvertRequest, images: Vec<ImageRef>) -> ConvertResult {
        let theme = self.themes.get(&req.theme).cloned();
        let mut html = render_local_html(&req.markdown, theme.as_ref());
        html = insert_image_placeholders(&html, &images);

        ConvertResult {
            html: Some(html),
            mode: ConvertMode::Local,
            theme: req.theme,
            images,
            status: ResultStatus::Completed,
            retryable: false,
            prompt: None,
            error: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct ApiConvertRequest {
    markdown: String,
    theme: String,
    #[serde(rename = "fontSize", skip_serializing_if = "Option::is_none")]
    font_size: Option<String>,
    #[serde(rename = "backgroundType", skip_serializing_if = "Option::is_none")]
    background_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiConvertResponse {
    code: i32,
    msg: String,
    data: ApiConvertResponseData,
}

#[derive(Debug, Deserialize)]
struct ApiConvertResponseData {
    html: String,
}

fn resolve_md2wechat_convert_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/api/convert") {
        base.to_string()
    } else {
        format!("{base}/api/convert")
    }
}

static IMAGE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"!\[[^\]]*\]\((__generate:[^)]+__|<[^>]+>|[^)\s]+)(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\)"#)
        .expect("image regex")
});

static IMG_TAG_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<img\b[^>]*>").expect("img tag regex"));

pub fn parse_markdown_images(markdown: &str) -> Vec<ImageRef> {
    let mut images = Vec::new();
    for cap in IMAGE_PATTERN.captures_iter(markdown) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let ref_normalized = normalize_image_reference(raw);
        if ref_normalized.is_empty() {
            continue;
        }

        let index = images.len();
        let placeholder = format!("<!-- IMG:{index} -->");
        let (kind, original, ai_prompt) =
            if ref_normalized.starts_with("http://") || ref_normalized.starts_with("https://") {
                (ImageKind::Remote, ref_normalized, None)
            } else if ref_normalized.starts_with("__generate:") && ref_normalized.ends_with("__") {
                let prompt = ref_normalized
                    .trim_start_matches("__generate:")
                    .trim_end_matches("__")
                    .trim()
                    .to_string();
                (ImageKind::Ai, prompt.clone(), Some(prompt))
            } else {
                (ImageKind::Local, ref_normalized, None)
            };

        images.push(ImageRef {
            index,
            original,
            placeholder,
            wechat_url: None,
            kind,
            ai_prompt,
        });
    }
    images
}

fn normalize_image_reference(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('<') && trimmed.ends_with('>') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

static FRONT_MATTER_DELIM: &str = "---";

#[derive(Debug, Default, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    author: Option<String>,
    digest: Option<String>,
    summary: Option<String>,
    description: Option<String>,
}

pub fn parse_article_metadata(markdown: &str) -> ArticleMetadata {
    let (fm, body) = parse_front_matter(markdown);
    let title = fm
        .as_ref()
        .and_then(|f| f.title.as_deref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| parse_markdown_title(&body));

    let author = fm
        .as_ref()
        .and_then(|f| f.author.as_deref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let digest = fm
        .as_ref()
        .and_then(|f| {
            first_non_empty(&[
                f.digest.as_deref(),
                f.summary.as_deref(),
                f.description.as_deref(),
            ])
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    ArticleMetadata {
        title,
        author,
        digest,
    }
}

fn parse_front_matter(markdown: &str) -> (Option<FrontMatter>, String) {
    let normalized = markdown.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let first = match lines.next() {
        Some(line) => line.trim(),
        None => return (None, markdown.to_string()),
    };
    if first != FRONT_MATTER_DELIM {
        return (None, markdown.to_string());
    }

    let mut fm_lines = Vec::new();
    let mut rest_lines = Vec::new();
    let mut in_fm = true;
    for line in lines {
        if in_fm && line.trim() == FRONT_MATTER_DELIM {
            in_fm = false;
            continue;
        }
        if in_fm {
            fm_lines.push(line);
        } else {
            rest_lines.push(line);
        }
    }

    if in_fm {
        return (None, markdown.to_string());
    }

    let fm_raw = fm_lines.join("\n");
    let fm = serde_yaml::from_str::<FrontMatter>(&fm_raw).ok();
    (fm, rest_lines.join("\n"))
}

pub fn parse_markdown_title(markdown: &str) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
        if !trimmed.is_empty() && !trimmed.starts_with('!') && !trimmed.starts_with('>') {
            return trimmed.to_string();
        }
    }
    "未命名文章".to_string()
}

fn first_non_empty<'a>(candidates: &[Option<&'a str>]) -> Option<&'a str> {
    candidates
        .iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
        .copied()
}

pub fn insert_image_placeholders(html: &str, images: &[ImageRef]) -> String {
    let mut result = html.to_string();
    let mut inserted: HashMap<usize, bool> = HashMap::new();

    for img in images {
        if img.placeholder.is_empty() {
            continue;
        }

        let candidates = [&img.original];
        for candidate in candidates {
            let quoted = regex::escape(candidate);
            let double = Regex::new(&format!(r#"(?i)<img[^>]*src="{quoted}"[^>]*>"#)).ok();
            let single = Regex::new(&format!(r#"(?i)<img[^>]*src='{quoted}'[^>]*>"#)).ok();
            if let Some(re) = &double {
                if re.is_match(&result) {
                    inserted.insert(img.index, true);
                }
                result = re
                    .replace_all(&result, img.placeholder.as_str())
                    .into_owned();
            }
            if let Some(re) = &single {
                if re.is_match(&result) {
                    inserted.insert(img.index, true);
                }
                result = re
                    .replace_all(&result, img.placeholder.as_str())
                    .into_owned();
            }
        }
    }

    for img in images {
        if inserted.get(&img.index).copied().unwrap_or(false) || img.placeholder.is_empty() {
            continue;
        }
        let placeholder = img.placeholder.clone();
        let mut done = false;
        result = IMG_TAG_PATTERN
            .replace_all(&result, |_: &regex::Captures| {
                if done {
                    return String::new();
                }
                done = true;
                inserted.insert(img.index, true);
                placeholder.clone()
            })
            .into_owned();
    }

    result
}

pub fn replace_image_placeholders(html: &str, images: &[ImageRef]) -> String {
    let mut result = html.to_string();
    for img in images {
        let Some(url) = img.wechat_url.as_deref() else {
            continue;
        };
        if !img.placeholder.is_empty() {
            let img_tag = format!(
                r#"<img src="{url}" style="max-width:100%;height:auto;display:block;margin:20px auto;" />"#
            );
            result = result.replace(&img.placeholder, &img_tag);
        }
        result = result.replace(
            &format!(r#"src="{}""#, img.original),
            &format!(r#"src="{url}""#),
        );
        result = result.replace(
            &format!(r#"src='{}'"#, img.original),
            &format!(r#"src='{url}'"#),
        );
    }
    result
}

fn build_ai_prompt(
    themes: &ThemeManager,
    theme_name: &str,
    markdown: &str,
    custom_prompt: Option<String>,
) -> String {
    let metadata = parse_article_metadata(markdown);
    let base = if let Some(custom) = custom_prompt {
        build_custom_ai_prompt(&custom)
    } else {
        match themes.get(theme_name) {
            Some(theme) if theme.kind() == ThemeKind::Ai => {
                theme.prompt.clone().unwrap_or_else(generic_ai_prompt)
            }
            _ => generic_ai_prompt(),
        }
    };

    let mut prompt = base;
    prompt = prompt.replace("{{TITLE}}", &metadata.title);
    if prompt.contains("{{MARKDOWN}}") {
        prompt = prompt.replace("{{MARKDOWN}}", markdown);
    } else {
        prompt.push_str("\n\n```\n");
        prompt.push_str(markdown);
        prompt.push_str("\n```");
    }
    prompt
}

fn build_custom_ai_prompt(custom_prompt: &str) -> String {
    let mut prompt = custom_prompt.to_string();
    let has_rules = prompt.contains("重要规则") || prompt.contains("规则");
    if !has_rules {
        prompt.push_str(
            "\n\n## 重要规则\n1. 所有 CSS 必须使用内联 style 属性\n2. 不使用外部样式表或 <style> 标签\n3. 只使用安全的 HTML 标签（section, p, span, strong, em, a, h1-h6, ul, ol, li, blockquote, pre, code, table, img, br, hr）\n4. 图片使用占位符格式：<!-- IMG:index -->\n5. 返回完整的 HTML，不需要其他说明文字\n",
        );
    }
    if !prompt.contains("请转换") {
        prompt.push_str("\n\n请转换以下 Markdown内容：");
    }
    prompt
}

fn generic_ai_prompt() -> String {
    "你是一个专业的微信公众号排版助手。请将以下 Markdown 内容转换为微信公众号兼容的 HTML。\n\n## 样式要求\n- 使用内联 CSS（style 属性）\n- 整洁大方的排版\n- 适当的间距和行高\n\n## 重要规则\n1. 所有 CSS 必须使用内联 style 属性\n2. 不使用外部样式表或 <style> 标签\n3. 只使用安全的 HTML 标签\n4. 图片使用占位符格式：<!-- IMG:index -->\n5. 返回完整的 HTML，不需要其他说明文字".to_string()
}

#[derive(Debug, Clone)]
struct ResolvedTheme {
    background: String,
    text: String,
    primary: String,
    quote_background: String,
    code_background: String,
    font_family: String,
    body_size: String,
    line_height: String,
    letter_spacing: Option<String>,
    link_underline: bool,
    heading_align: Option<String>,
    h1_background: Option<String>,
    h1_radius: Option<String>,
    container_padding: String,
    container_max_width: Option<String>,
    container_center: bool,
    card_enabled: bool,
    card_padding: String,
    card_radius: String,
    card_background: String,
    card_border: String,
    card_shadow: String,
    background_texture: Option<String>,
    h1_size: String,
    h2_size: String,
    h3_size: String,
    list_marker: String,
}

fn resolve_theme(theme: Option<&Theme>) -> ResolvedTheme {
    let palette = theme.and_then(|t| t.palette.as_ref());
    let typography = theme.and_then(|t| t.typography.as_ref());
    let layout = theme.and_then(|t| t.layout.as_ref());

    let background =
        palette_value(theme, palette, "background").unwrap_or_else(|| "#ffffff".to_string());
    let text = palette_value(theme, palette, "text").unwrap_or_else(|| "#222222".to_string());
    let primary = palette_value(theme, palette, "primary").unwrap_or_else(|| "#4a7c9b".to_string());
    let quote_background =
        palette_value(theme, palette, "quote_background").unwrap_or_else(|| "#f5f5f5".to_string());
    let code_background = palette
        .and_then(|p| p.code_background.clone())
        .unwrap_or_else(|| "#0b1020".to_string());

    let font_family = typography
        .and_then(|t| t.font_family.clone())
        .unwrap_or_else(|| {
            "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"
                .to_string()
        });
    let body_size = typography
        .and_then(|t| t.body_size.clone())
        .unwrap_or_else(|| "16px".to_string());
    let line_height = typography
        .and_then(|t| t.line_height.clone())
        .unwrap_or_else(|| "1.75".to_string());
    let letter_spacing = typography.and_then(|t| t.letter_spacing.clone());
    let link_underline = typography.and_then(|t| t.link_underline).unwrap_or(false);
    let heading_align = typography.and_then(|t| t.heading_align.clone());
    let h1_background = typography.and_then(|t| t.h1_background.clone());
    let h1_radius = typography.and_then(|t| t.h1_radius.clone());
    let h1_size = typography
        .and_then(|t| t.h1_size.clone())
        .unwrap_or_else(|| "22px".to_string());
    let h2_size = typography
        .and_then(|t| t.h2_size.clone())
        .unwrap_or_else(|| "20px".to_string());
    let h3_size = typography
        .and_then(|t| t.h3_size.clone())
        .unwrap_or_else(|| "18px".to_string());

    let container_padding = layout
        .and_then(|l| l.container.as_ref())
        .and_then(|c| c.padding.clone())
        .unwrap_or_else(|| "40px 10px".to_string());
    let container_max_width = layout
        .and_then(|l| l.container.as_ref())
        .and_then(|c| c.max_width.clone());
    let container_center = layout
        .and_then(|l| l.container.as_ref())
        .and_then(|c| c.center)
        .unwrap_or(true);

    let card_enabled = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.enabled)
        .unwrap_or(false);
    let card_padding = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.padding.clone())
        .unwrap_or_else(|| "22px".to_string());
    let card_radius = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.radius.clone())
        .unwrap_or_else(|| "16px".to_string());
    let card_background = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.background.clone())
        .unwrap_or_else(|| "#ffffff".to_string());
    let card_border = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.border.clone())
        .unwrap_or_else(|| "1px solid rgba(0,0,0,0.06)".to_string());
    let card_shadow = layout
        .and_then(|l| l.card.as_ref())
        .and_then(|c| c.shadow.clone())
        .unwrap_or_else(|| "0 10px 30px rgba(0,0,0,0.06)".to_string());

    let background_texture = layout
        .and_then(|l| l.decorations.as_ref())
        .and_then(|d| d.background_texture.clone())
        .filter(|v| v != "none");

    let list_marker = layout
        .and_then(|l| l.list_marker.clone())
        .unwrap_or_else(|| "●".to_string());

    ResolvedTheme {
        background,
        text,
        primary,
        quote_background,
        code_background,
        font_family,
        body_size,
        line_height,
        letter_spacing,
        link_underline,
        heading_align,
        h1_background,
        h1_radius,
        container_padding,
        container_max_width,
        container_center,
        card_enabled,
        card_padding,
        card_radius,
        card_background,
        card_border,
        card_shadow,
        background_texture,
        h1_size,
        h2_size,
        h3_size,
        list_marker,
    }
}

fn palette_value(theme: Option<&Theme>, palette: Option<&Palette>, key: &str) -> Option<String> {
    if let Some(p) = palette {
        let from_palette = match key {
            "background" => p.background.clone(),
            "text" => p.text.clone(),
            "primary" => p.primary.clone(),
            "secondary" => p.secondary.clone(),
            "muted" => p.muted.clone(),
            "quote_background" => p.quote_background.clone(),
            "code_background" => p.code_background.clone(),
            _ => None,
        };
        if from_palette.is_some() {
            return from_palette;
        }
    }
    theme
        .and_then(|t| t.colors.as_ref())
        .and_then(|map| map.get(key))
        .cloned()
}

fn render_local_html(markdown: &str, theme: Option<&Theme>) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut inner = String::new();
    html::push_html(&mut inner, parser);

    let resolved = resolve_theme(theme);
    let inner = apply_basic_inline_styles(&inner, &resolved);

    let mut container_style = format!(
        "background-color:{};padding:{};font-family:{};",
        resolved.background, resolved.container_padding, resolved.font_family
    );
    if let Some(letter_spacing) = resolved.letter_spacing.as_deref() {
        container_style.push_str(&format!("letter-spacing:{};", letter_spacing));
    }
    if let Some(texture) = resolved.background_texture.as_deref() {
        container_style.push_str(&background_texture_css(texture));
    }

    let content = if resolved.container_center {
        if let Some(max_width) = resolved.container_max_width.as_deref() {
            format!(r#"<div style="max-width:{max_width};margin:0 auto;">{inner}</div>"#)
        } else {
            inner
        }
    } else {
        inner
    };

    let content = if resolved.card_enabled {
        format!(
            r#"<section style="background-color:{};padding:{};border-radius:{};border:{};box-shadow:{};">{}</section>"#,
            resolved.card_background,
            resolved.card_padding,
            resolved.card_radius,
            resolved.card_border,
            resolved.card_shadow,
            content
        )
    } else {
        content
    };

    format!(r#"<div style="{container_style}">{content}</div>"#)
}

fn apply_basic_inline_styles(html: &str, theme: &ResolvedTheme) -> String {
    let mut result = html.to_string();

    result = result.replace(
        "<p>",
        &format!(
            r#"<p style="margin:12px 0;color:{};font-size:{};line-height:{};">"#,
            theme.text, theme.body_size, theme.line_height
        ),
    );

    for level in 1..=6 {
        let open = format!("<h{level}>");
        let (size, margin) = match level {
            1 => (&theme.h1_size, "22px 0 12px"),
            2 => (&theme.h2_size, "22px 0 12px"),
            3 => (&theme.h3_size, "18px 0 10px"),
            _ => (&theme.h3_size, "18px 0 10px"),
        };
        let mut style = format!(
            "margin:{};color:{};font-size:{};line-height:{};",
            margin, theme.primary, size, "1.35"
        );
        if let Some(align) = theme.heading_align.as_deref() {
            style.push_str(&format!("text-align:{};", align));
        }
        if level == 1 {
            if let Some(bg) = theme.h1_background.as_deref() {
                let radius = theme.h1_radius.as_deref().unwrap_or("10px");
                style.push_str(&format!(
                    "background-color:{};color:#ffffff;padding:10px 12px;border-radius:{};",
                    bg, radius
                ));
            }
        } else if level <= 3 {
            style.push_str(&format!(
                "background-color:{};padding:8px 12px;border-radius:12px;border-left:4px solid {};box-sizing:border-box;",
                theme.quote_background, theme.primary
            ));
        }
        let styled = format!(r#"<h{level} style="{style}">"#);
        result = result.replace(&open, &styled);
    }

    result = result.replace(
        "<blockquote>",
        &format!(
            r#"<blockquote style="margin:16px 0;padding:12px 14px;border-left:4px solid {};background-color:{};">"#,
            theme.primary, theme.quote_background
        ),
    );

    let link_open = Regex::new(r#"(?i)<a href="([^"]+)">"#).expect("a regex");
    let underline = if theme.link_underline {
        "underline"
    } else {
        "none"
    };
    result = link_open
        .replace_all(&result, |cap: &regex::Captures| {
            format!(
                r#"<a href="{}" style="color:{};text-decoration:{};">"#,
                &cap[1], theme.primary, underline
            )
        })
        .into_owned();

    let code_block_open =
        Regex::new(r#"(?is)<pre><code([^>]*)>"#).expect("pre code open regex");
    result = code_block_open
        .replace_all(&result, |cap: &regex::Captures| {
            let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            format!(
                r#"<pre style="margin:16px 0;padding:12px 14px;background-color:{};border-radius:10px;overflow:auto;"><code data-mpa="codeblock"{} style="color:#e6edf3;font-size:13px;line-height:1.6;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,'Liberation Mono','Courier New',monospace;tab-size:2;">"#,
                theme.code_background, attrs
            )
        })
        .into_owned();

    let code_open = Regex::new(r#"(?is)<code([^>]*)>"#).expect("code open regex");
    result = code_open
        .replace_all(&result, |cap: &regex::Captures| {
            let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let attrs_lower = attrs.to_ascii_lowercase();
            if attrs_lower.contains(r#"data-mpa="codeblock""#) || attrs_lower.contains("style=") {
                return cap[0].to_string();
            }
            format!(
                r#"<code{} style="background-color:{};color:{};padding:2px 6px;border-radius:8px;font-size:0.92em;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,'Liberation Mono','Courier New',monospace;">"#,
                attrs, theme.quote_background, theme.primary
            )
        })
        .into_owned();

    let ul_block = Regex::new(r#"(?is)<ul([^>]*)>(.*?)</ul>"#).expect("ul block regex");
    result = ul_block
        .replace_all(&result, |cap: &regex::Captures| {
            let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let inner = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            
            // 清除所有的单独换行，这常常导致多余的空行
            let inner_str = inner.replace("\n", "");
            
            // 首先清理完全空的 li
            let empty_li = Regex::new(r#"(?is)<li[^>]*>\s*</li>"#).expect("empty li regex");
            let mut inner = empty_li.replace_all(&inner_str, "").into_owned();
            
            // 匹配并替换 <li>...</li> 以便我们在内部正确插入符号和样式
            // 我们不能简单地在 <li> 后面插入内容，因为微信可能对结构敏感。
            // 更好的做法是，对于 <li> 的内容，如果是纯文本，直接包一层 span；如果有 p，确保 p 的 margin 为 0。
            let li_block = Regex::new(r#"(?is)<li([^>]*)>(.*?)</li>"#).expect("li block regex");
            inner = li_block
                .replace_all(&inner, |c2: &regex::Captures| {
                    let li_attrs = c2.get(1).map(|m| m.as_str()).unwrap_or("");
                    let li_content = c2.get(2).map(|m| m.as_str()).unwrap_or("");
                    let li_attrs_lower = li_attrs.to_ascii_lowercase();
                    
                    // 移除 li 内容中的换行符和多余空格，这经常导致微信出现空行
                    let cleaned_content = li_content.trim();
                    
                    let style_attr = if li_attrs_lower.contains("style=") {
                        li_attrs.to_string()
                    } else {
                        format!(
                            r#"{} style="margin:6px 0;padding-left:18px;position:relative;color:{};font-size:{};line-height:{};""#,
                            li_attrs, theme.text, theme.body_size, theme.line_height
                        )
                    };
                    
                    // 我们使用一个简单的 span 而不是将符号直接放在内容前面，
                    // 以免打乱微信对后续 <p> 的解析。
                    // 并且我们移除了 span 的换行。
                    format!(
                        r#"<li{}><span style="position:absolute;left:0;top:0;color:{};font-weight:700;font-size:0.9em;line-height:{};">{}</span>{}"#,
                        style_attr, theme.primary, theme.line_height, theme.list_marker, cleaned_content
                    )
                })
                .into_owned();
                
            // 对于 li 内部可能存在的 <p>，我们不再保留 <p> 标签（微信很容易在这上面折腾出换行）
            // 我们将 <p> 转换为 span，或者直接去掉 <p> 的外壳
            let p_block = Regex::new(r#"(?is)<p[^>]*>(.*?)</p>"#).expect("p block regex");
            inner = p_block
                .replace_all(&inner, |c: &regex::Captures| {
                    let p_content = c.get(1).map(|m| m.as_str()).unwrap_or("");
                    format!(r#"<span style="display:inline-block;">{}</span>"#, p_content.trim())
                })
                .into_owned();
                
            // 为了防止微信识别出多余的空行，我们还要将 </li> 标签后的换行去掉
            format!(
                r#"<ul{} style="margin:12px 0;padding:0;list-style-type:none;">{}</ul>"#,
                attrs, inner
            )
        })
        .into_owned();

    let ol_open = Regex::new(r#"(?i)<ol>"#).expect("ol open regex");
    result = ol_open
        .replace_all(
            &result,
            &format!(
                r#"<ol style="margin:12px 0;padding-left:22px;color:{};font-size:{};line-height:{};">"#,
                theme.text, theme.body_size, theme.line_height
            ),
        )
        .into_owned();
    let li_p_margin = Regex::new(r#"(?is)<li([^>]*)>\s*<p style="margin:12px 0;"#)
        .expect("li p margin regex");
    result = li_p_margin
        .replace_all(&result, r#"<li$1><p style="margin:0;"#)
        .into_owned();
    let empty_li = Regex::new(r#"(?is)<li[^>]*>\s*</li>"#).expect("empty li regex");
    result = empty_li.replace_all(&result, "").into_owned();

    let hr_pattern = Regex::new(r"(?i)<hr\s*/?>").expect("hr regex");
    result = hr_pattern
        .replace_all(
            &result,
            r#"<hr style="border:none;height:1px;background-color:rgba(0,0,0,0.12);margin:18px 0;" />"#,
        )
        .into_owned();

    result
}

fn background_texture_css(kind: &str) -> String {
    match kind {
        "grid" => "background-image: linear-gradient(rgba(0,0,0,0.035) 1px, transparent 1px), linear-gradient(90deg, rgba(0,0,0,0.035) 1px, transparent 1px);background-size: 22px 22px;".to_string(),
        "dot" => "background-image: radial-gradient(rgba(0,0,0,0.06) 1px, transparent 1px);background-size: 16px 16px;".to_string(),
        "lines" => "background-image: linear-gradient(rgba(0,0,0,0.04) 1px, transparent 1px);background-size: 24px 24px;".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_front_matter_metadata() {
        let md = "---\ntitle: T\nauthor: A\nsummary: S\n---\n\n# H\nbody";
        let meta = parse_article_metadata(md);
        assert_eq!(meta.title, "T");
        assert_eq!(meta.author.as_deref(), Some("A"));
        assert_eq!(meta.digest.as_deref(), Some("S"));
    }

    #[test]
    fn parses_images_and_generates_placeholders() {
        let md = "![a](./a.png)\n![b](https://example.com/b.png)\n![c](__generate:draw fox__)";
        let images = parse_markdown_images(md);
        assert_eq!(images.len(), 3);
        assert_eq!(images[0].kind, ImageKind::Local);
        assert_eq!(images[1].kind, ImageKind::Remote);
        assert_eq!(images[2].kind, ImageKind::Ai);
        assert_eq!(images[2].ai_prompt.as_deref(), Some("draw fox"));
        assert_eq!(images[0].placeholder, "<!-- IMG:0 -->");
    }

    #[test]
    fn inserts_and_replaces_placeholders() {
        let html = r#"<p>x</p><img src="./a.png"><p>y</p><img src="https://example.com/b.png">"#;
        let mut images = vec![
            ImageRef {
                index: 0,
                original: "./a.png".to_string(),
                placeholder: "<!-- IMG:0 -->".to_string(),
                wechat_url: Some("https://wechat.local/a".to_string()),
                kind: ImageKind::Local,
                ai_prompt: None,
            },
            ImageRef {
                index: 1,
                original: "https://example.com/b.png".to_string(),
                placeholder: "<!-- IMG:1 -->".to_string(),
                wechat_url: Some("https://wechat.local/b".to_string()),
                kind: ImageKind::Remote,
                ai_prompt: None,
            },
        ];

        let with_placeholders = insert_image_placeholders(html, &images);
        assert!(with_placeholders.contains("<!-- IMG:0 -->"));
        assert!(with_placeholders.contains("<!-- IMG:1 -->"));

        images[0].wechat_url = Some("https://wechat.local/a".to_string());
        let replaced = replace_image_placeholders(&with_placeholders, &images);
        assert!(replaced.contains("https://wechat.local/a"));
        assert!(replaced.contains("https://wechat.local/b"));
        assert!(!replaced.contains("./a.png"));
    }
}
