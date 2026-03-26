use super::{CardLayout, ContainerLayout, Decorations, Layout, Palette, Theme, Typography};
use std::collections::HashMap;

pub fn builtin_themes() -> Vec<Theme> {
    let mut themes = Vec::new();

    themes.extend(base_themes());
    themes.extend(series_themes());

    themes
}

fn base_themes() -> Vec<Theme> {
    vec![
        local(
            "default",
            "经典 · 温暖",
            palette("#faf9f5", "#4a413d", "#d97758", Some("#c06b4d"), Some("#fef4e7")),
            typography(None, None, None, None, Some(false), None),
            layout_container(Some("40px 10px"), Some("820px"), Some(true), None, None),
        )
        .with_list_marker("●"),
        local(
            "bytedance",
            "科技 · 现代",
            palette("#f7f9fc", "#1f2328", "#1677ff", Some("#00b578"), Some("#eef5ff")),
            typography(None, None, None, Some("0.2px"), Some(false), None),
            layout_container(Some("40px 10px"), Some("860px"), Some(true), None, None),
        )
        .with_list_marker("▪"),
        local(
            "apple",
            "视觉 · 渐变",
            palette("#ffffff", "#111827", "#7c3aed", Some("#2563eb"), Some("#f5f3ff")),
            typography(
                None,
                Some("16px"),
                Some("1.8"),
                Some("0.2px"),
                Some(false),
                Some("center"),
            ),
            layout_container(
                Some("44px 12px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("26px"), Some("18px"))),
                Some(decor("none")),
            ),
        )
        .with_list_marker("○"),
        local(
            "sports",
            "活力 · 动感",
            palette("#0b1220", "#e5e7eb", "#22c55e", Some("#f97316"), Some("#111827")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.75"),
                Some("0.3px"),
                Some(false),
                Some("left"),
            )
            .with_h1_background("#22c55e", "10px"),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("16px"))),
                Some(decor("none")),
            ),
        )
        .with_list_marker("►"),
        local(
            "chinese",
            "古典 · 雅致",
            palette("#fbfaf8", "#2d2a26", "#8b5a2b", Some("#b7791f"), Some("#f3efe7")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.85"),
                Some("0.6px"),
                Some(false),
                Some("left"),
            ),
            layout_container(
                Some("40px 10px"),
                Some("780px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("14px"))),
                Some(decor("grid")),
            ),
        )
        .with_list_marker("❖"),
        local(
            "cyber",
            "未来 · 霓虹 · 科技",
            palette("#0a0b10", "#e5e7eb", "#a855f7", Some("#22d3ee"), Some("#111827")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.75"),
                Some("0.4px"),
                Some(false),
                Some("left"),
            ),
            layout_container(
                Some("40px 10px"),
                Some("900px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("16px"))),
                Some(decor("grid")),
            ),
        )
        .with_list_marker("⚡"),
        local(
            "wechat-native",
            "原汁原味官方绿底纹",
            palette("#eaf5ea", "#1f3b1f", "#2f855a", Some("#16a34a"), Some("#def7e1")),
            typography(None, None, None, Some("0.2px"), Some(false), None),
            layout_container(Some("40px 10px"), Some("780px"), Some(true), None, Some(decor("grid"))),
        )
        .with_list_marker("●"),
        local(
            "nyt-classic",
            "经典米黄色新闻纸",
            palette("#fbf3d3", "#2d2a26", "#8b5a2b", Some("#6b7280"), Some("#fff7df")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.9"),
                Some("0.4px"),
                Some(true),
                Some("left"),
            ),
            layout_container(
                Some("44px 12px"),
                Some("760px"),
                Some(true),
                Some(card(Some(true), Some("26px"), Some("10px"))),
                Some(decor("lines")),
            ),
        )
        .with_list_marker("■"),
        local(
            "github-readme",
            "README 即视感",
            palette("#ffffff", "#24292e", "#0969da", Some("#57606a"), Some("#f6f8fa")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.75"),
                Some("0.2px"),
                Some(false),
                Some("left"),
            ),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("14px"))),
                None,
            ),
        )
        .with_list_marker("•"),
        local(
            "sspai-red",
            "数字媒体红色标识",
            palette("#ffffff", "#111827", "#dc2626", Some("#ef4444"), Some("#fff1f2")),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left"))
                .with_h1_background("#dc2626", "10px"),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("16px"))),
                None,
            ),
        )
        .with_list_marker("✦"),
        local(
            "mint-fresh",
            "清凉薄荷绿底色",
            palette("#ecfdf5", "#064e3b", "#10b981", Some("#34d399"), Some("#d1fae5")),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("820px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("16px"))),
                Some(decor("dot")),
            ),
        )
        .with_list_marker("🌿"),
        local(
            "sunset-amber",
            "暖琥珀黄昏意境",
            palette("#fff7ed", "#4a2c2a", "#f59e0b", Some("#fb7185"), Some("#ffedd5")),
            typography(None, None, None, Some("0.3px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("18px"))),
                Some(decor("none")),
            ),
        )
        .with_list_marker("-"),
        local(
            "ink-minimal",
            "黑白水墨极简阅读",
            palette("#ffffff", "#111111", "#111111", Some("#4b5563"), Some("#f3f4f6")),
            typography(
                Some("-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif"),
                Some("16px"),
                Some("1.95"),
                Some("0px"),
                Some(false),
                Some("left"),
            ),
            layout_container(Some("40px 10px"), Some("760px"), Some(true), None, None),
        )
        .with_list_marker("▪"),
        local(
            "paper-white",
            "纯白纸感",
            palette("#ffffff", "#111827", "#334155", Some("#64748b"), Some("#f1f5f9")),
            typography(None, None, None, Some("0.1px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("820px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("14px"))),
                None,
            ),
        )
        .with_list_marker("○"),
        local(
            "midnight-dark",
            "午夜深色阅读",
            palette("#0b1020", "#e6edf3", "#22d3ee", Some("#a855f7"), Some("#111827")),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("22px"), Some("16px"))),
                None,
            ),
        )
        .with_list_marker("✦"),
        local(
            "lavender-dream",
            "薰衣草梦幻",
            palette("#faf5ff", "#2d1b3d", "#8b5cf6", Some("#ec4899"), Some("#f3e8ff")),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("18px"))),
                Some(decor("dot")),
            ),
        )
        .with_list_marker("★"),
    ]
}

fn series_themes() -> Vec<Theme> {
    let colors = [
        ("gold", "#b88a00", "#fffaf0", "#3b2f1a"),
        ("green", "#2f855a", "#f0fff4", "#1f3b2f"),
        ("blue", "#2b6cb0", "#eff6ff", "#1e3a5f"),
        ("orange", "#dd6b20", "#fff7ed", "#4a2c2a"),
        ("red", "#c53030", "#fff5f5", "#3b1d1d"),
        ("navy", "#2c5282", "#eff6ff", "#1e2a44"),
        ("gray", "#4a5568", "#f7fafc", "#1f2937"),
        ("sky", "#3182ce", "#f0f9ff", "#1e3a5f"),
    ];

    let mut themes = Vec::new();
    for (suffix, primary, background, text) in colors {
        let quote = "#ffffff";

        themes.push(local(
            &format!("minimal-{suffix}"),
            "干净克制，纯色文字无装饰",
            palette(background, text, primary, None, Some(quote)),
            typography(None, None, None, Some("0.1px"), Some(false), Some("left")),
            layout_container(Some("40px 10px"), Some("860px"), Some(true), None, None),
        ));

        themes.push(local(
            &format!("focus-{suffix}"),
            "居中对称，标题上下双横线",
            palette(background, text, primary, None, Some(quote)),
            typography(None, None, None, Some("0.3px"), Some(false), Some("center")),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("16px"))),
                Some(decor("none")),
            ),
        ));

        themes.push(local(
            &format!("elegant-{suffix}"),
            "层次丰富，左边框递减 + 渐变背景",
            palette(background, text, primary, None, Some(quote)),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left")),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("26px"), Some("18px"))),
                Some(decor("grid")),
            ),
        ));

        themes.push(local(
            &format!("bold-{suffix}"),
            "视觉冲击，标题满底色 + 圆角投影",
            palette(background, text, primary, None, Some(quote)),
            typography(None, None, None, Some("0.2px"), Some(false), Some("left"))
                .with_h1_background(primary, "12px"),
            layout_container(
                Some("40px 10px"),
                Some("860px"),
                Some(true),
                Some(card(Some(true), Some("24px"), Some("18px"))),
                Some(decor("none")),
            ),
        ));
    }

    themes
}

fn palette(
    background: &str,
    text: &str,
    primary: &str,
    secondary: Option<&str>,
    quote_background: Option<&str>,
) -> Palette {
    Palette {
        background: Some(background.to_string()),
        text: Some(text.to_string()),
        primary: Some(primary.to_string()),
        secondary: secondary.map(|v| v.to_string()),
        muted: None,
        quote_background: quote_background.map(|v| v.to_string()),
        code_background: None,
    }
}

fn typography(
    font_family: Option<&str>,
    body_size: Option<&str>,
    line_height: Option<&str>,
    letter_spacing: Option<&str>,
    link_underline: Option<bool>,
    heading_align: Option<&str>,
) -> Typography {
    Typography {
        font_family: font_family.map(|v| v.to_string()),
        body_size: body_size.map(|v| v.to_string()),
        line_height: line_height.map(|v| v.to_string()),
        letter_spacing: letter_spacing.map(|v| v.to_string()),
        link_underline,
        heading_align: heading_align.map(|v| v.to_string()),
        h1_size: None,
        h2_size: None,
        h3_size: None,
        h1_background: None,
        h1_radius: None,
    }
}

trait TypographyExt {
    fn with_h1_background(self, background: &str, radius: &str) -> Self;
}

impl TypographyExt for Typography {
    fn with_h1_background(mut self, background: &str, radius: &str) -> Self {
        self.h1_background = Some(background.to_string());
        self.h1_radius = Some(radius.to_string());
        self
    }
}

fn layout_container(
    padding: Option<&str>,
    max_width: Option<&str>,
    center: Option<bool>,
    card: Option<CardLayout>,
    decorations: Option<Decorations>,
) -> Layout {
    Layout {
        container: Some(ContainerLayout {
            padding: padding.map(|v| v.to_string()),
            max_width: max_width.map(|v| v.to_string()),
            center,
        }),
        card,
        decorations,
        list_marker: None,
    }
}

fn card(enabled: Option<bool>, padding: Option<&str>, radius: Option<&str>) -> CardLayout {
    CardLayout {
        enabled,
        padding: padding.map(|v| v.to_string()),
        radius: radius.map(|v| v.to_string()),
        background: Some("#ffffff".to_string()),
        border: Some("1px solid rgba(0,0,0,0.06)".to_string()),
        shadow: Some("0 10px 30px rgba(0,0,0,0.06)".to_string()),
    }
}

fn decor(kind: &str) -> Decorations {
    Decorations {
        background_texture: Some(kind.to_string()),
    }
}

fn local(
    name: &str,
    description: &str,
    palette: Palette,
    typography: Typography,
    layout: Layout,
) -> Theme {
    let mut colors = HashMap::new();
    if let Some(v) = palette.background.clone() {
        colors.insert("background".to_string(), v);
    }
    if let Some(v) = palette.text.clone() {
        colors.insert("text".to_string(), v);
    }
    if let Some(v) = palette.primary.clone() {
        colors.insert("primary".to_string(), v);
    }
    if let Some(v) = palette.secondary.clone() {
        colors.insert("secondary".to_string(), v);
    }
    if let Some(v) = palette.quote_background.clone() {
        colors.insert("quote_background".to_string(), v);
    }

    Theme {
        name: name.to_string(),
        r#type: Some("local".to_string()),
        description: Some(description.to_string()),
        version: Some("1.0".to_string()),
        colors: Some(colors),
        palette: Some(palette),
        typography: Some(typography),
        layout: Some(layout),
        api_theme: None,
        prompt: None,
    }
}

trait ThemeExt {
    fn with_list_marker(self, marker: &str) -> Self;
}

impl ThemeExt for Theme {
    fn with_list_marker(mut self, marker: &str) -> Self {
        if let Some(layout) = &mut self.layout {
            layout.list_marker = Some(marker.to_string());
        }
        self
    }
}
