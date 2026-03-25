use mpa_core::converter::{ConvertMode, ConvertRequest, MarkdownConverter};
use mpa_core::platforms::wechat::WechatPublisher;
use mpa_core::platforms::{DraftArticle, Publisher};
use mpa_core::{tui, ApiConfig, Config, ThemeManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        tui::run()?;
        return Ok(());
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "tui" => {
            tui::run()?;
            Ok(())
        }
        "themes" => themes_cmd(args),
        "convert" => convert_cmd(args).await,
        "publish" => publish_cmd(args).await,
        _ => {
            eprintln!("unknown command: {cmd}");
            std::process::exit(2);
        }
    }
}

fn themes_cmd(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("usage: mpa themes <list|show>");
        std::process::exit(2);
    }
    let sub = &args[0];
    let tm = ThemeManager::new();
    match sub.as_str() {
        "list" => {
            for theme in tm.list() {
                println!("{}", theme.name);
            }
            Ok(())
        }
        "show" => {
            if args.len() < 2 {
                eprintln!("usage: mpa themes show <name>");
                std::process::exit(2);
            }
            let name = &args[1];
            let theme = tm
                .get(name)
                .ok_or_else(|| format!("theme not found: {name}"))?;
            let json = serde_json::to_string_pretty(theme)?;
            println!("{json}");
            Ok(())
        }
        _ => {
            eprintln!("unknown subcommand: {sub}");
            std::process::exit(2);
        }
    }
}

async fn convert_cmd(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("usage: mpa convert <markdown_file> [--mode local|api|ai] [--theme <name>] [-o <out.html>]");
        std::process::exit(2);
    }

    let input = args[0].clone();
    let mut mode = ConvertMode::Local;
    let mut theme = "default".to_string();
    let mut out: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                mode = match args.get(i).map(|s| s.as_str()) {
                    Some("local") => ConvertMode::Local,
                    Some("api") => ConvertMode::Api,
                    Some("ai") => ConvertMode::Ai,
                    Some(v) => return Err(format!("invalid mode: {v}").into()),
                    None => return Err("missing --mode value".into()),
                };
            }
            "--theme" => {
                i += 1;
                theme = args
                    .get(i)
                    .cloned()
                    .ok_or_else(|| "missing --theme value".to_string())?;
            }
            "-o" | "--out" => {
                i += 1;
                out = Some(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| "missing -o value".to_string())?,
                );
            }
            other => return Err(format!("unknown arg: {other}").into()),
        }
        i += 1;
    }

    let markdown = std::fs::read_to_string(&input)?;
    let cfg = Config::load();
    let themes = ThemeManager::new();
    let converter = MarkdownConverter::new(
        ApiConfig {
            md2wechat_api_key: cfg.api.md2wechat_api_key.clone(),
            md2wechat_base_url: cfg.api.md2wechat_base_url.clone(),
        },
        themes,
    );

    let result = converter
        .convert(ConvertRequest {
            markdown,
            mode,
            theme: theme.clone(),
            api_key: None,
            font_size: None,
            background_type: None,
            custom_prompt: None,
        })
        .await?;

    if let Some(prompt) = result.prompt.as_deref() {
        println!("{prompt}");
        return Ok(());
    }

    let html = result.html.ok_or("no html produced")?;
    if let Some(out) = out {
        std::fs::write(out, html)?;
    } else {
        println!("{html}");
    }
    Ok(())
}

async fn publish_cmd(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        eprintln!("usage: mpa publish wechat-draft --html <file.html> --title <title> --cover <cover.jpg> [--author <name>] [--digest <text>]");
        std::process::exit(2);
    }

    let sub = &args[0];
    match sub.as_str() {
        "wechat-draft" => publish_wechat_draft(args[1..].to_vec()).await,
        _ => Err(format!("unknown publish target: {sub}").into()),
    }
}

async fn publish_wechat_draft(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut html_path: Option<PathBuf> = None;
    let mut title: Option<String> = None;
    let mut cover: Option<PathBuf> = None;
    let mut author: Option<String> = None;
    let mut digest: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--html" => {
                i += 1;
                html_path = Some(PathBuf::from(args.get(i).ok_or("missing --html value")?));
            }
            "--title" => {
                i += 1;
                title = Some(args.get(i).cloned().ok_or("missing --title value")?);
            }
            "--cover" => {
                i += 1;
                cover = Some(PathBuf::from(args.get(i).ok_or("missing --cover value")?));
            }
            "--author" => {
                i += 1;
                author = Some(args.get(i).cloned().ok_or("missing --author value")?);
            }
            "--digest" => {
                i += 1;
                digest = Some(args.get(i).cloned().ok_or("missing --digest value")?);
            }
            other => return Err(format!("unknown arg: {other}").into()),
        }
        i += 1;
    }

    let cfg = Config::load();
    let wechat_cfg = cfg
        .wechat
        .ok_or("missing WECHAT_APPID/WECHAT_SECRET (or configure in mpa tui)")?;
    let publisher = WechatPublisher::new(wechat_cfg)?;

    let cover_path = cover.ok_or("missing --cover")?;
    let uploaded = publisher.upload_image_file(&cover_path).await?;

    let html_path = html_path.ok_or("missing --html")?;
    let content_html = std::fs::read_to_string(html_path)?;

    let title = title.ok_or("missing --title")?;
    let result = publisher
        .create_draft(vec![DraftArticle {
            title,
            author,
            digest,
            content_html,
            cover_media_id: Some(uploaded.media_id),
            show_cover_pic: true,
            content_source_url: None,
        }])
        .await?;

    println!("{}", result.media_id);
    Ok(())
}
