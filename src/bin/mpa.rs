use mpa_core::converter::{ConvertMode, ConvertRequest, MarkdownConverter};
use mpa_core::platforms::wechat::WechatPublisher;
use mpa_core::platforms::{DraftArticle, Publisher};
use mpa_core::publish::{AssetKind, AssetPipeline, AssetRef, ProcessInput};
use mpa_core::{tui, ApiConfig, Config, ThemeManager};
use regex::Regex;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        tui::run()?;
        return Ok(());
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" | "-v" => {
            println!("mpa {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "install" => install_cmd(args),
        "tui" => {
            tui::run()?;
            Ok(())
        }
        "themes" => themes_cmd(args),
        "convert" => convert_cmd(args).await,
        "publish" => publish_cmd(args).await,
        _ => {
            eprintln!("unknown command: {cmd}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        r#"mpa - Multi-Platform Articles

Usage:
  mpa                 Open TUI
  mpa tui             Open TUI
  mpa --version       Print version
  mpa --help          Print this help

Commands:
  install             Install binary and (optionally) skill
  themes              List/show themes
  convert             Convert markdown to HTML
  publish             Publish to platforms (wechat-draft supported)

Examples:
  mpa themes list
  mpa convert article.md --mode local --theme github-readme -o out.html
  mpa publish wechat-draft --md article.md --cover cover.jpg --mode local --theme github-readme
"#
    );
}

fn install_cmd(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut install_dir: Option<PathBuf> = None;
    let mut install_skill = true;
    let mut update_path = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                install_dir = Some(PathBuf::from(args.get(i).ok_or("missing --dir value")?));
            }
            "--no-skill" => install_skill = false,
            "--no-path" => update_path = false,
            other => return Err(format!("unknown arg: {other}").into()),
        }
        i += 1;
    }

    let exe = std::env::current_exe()?;
    let default_dir = default_install_dir();
    let install_dir = install_dir.unwrap_or(default_dir);
    std::fs::create_dir_all(&install_dir)?;

    let target = install_dir.join(exe.file_name().ok_or("invalid current exe")?);
    std::fs::copy(&exe, &target)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&target, perm)?;
    }

    println!("Installed binary: {}", target.display());

    if update_path {
        if let Err(err) = ensure_path_contains(&install_dir) {
            eprintln!("PATH update skipped: {err}");
        }
    }

    if install_skill {
        match install_skill_folder("multi-platform-articles") {
            Ok(Some(path)) => {
                println!("Installed skill for Trae: {}", path.display());
                println!("Note: If you use Cursor, skill was also copied to ~/.cursor/skills/ if the directory exists.");
            },
            Ok(None) => eprintln!("Skill not found in package; install from ClawHub/Trae instead."),
            Err(err) => eprintln!("Skill install skipped: {err}"),
        }
    }

    println!("Next: run `mpa` to open TUI and configure WECHAT_APPID/WECHAT_SECRET");
    Ok(())
}

fn default_install_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".local").join("bin");
        }
        PathBuf::from(".")
    } else {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".local").join("bin");
        }
        PathBuf::from(".")
    }
}

fn ensure_path_contains(install_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("PATH").unwrap_or_default();
    let install_dir_str = install_dir.to_string_lossy();
    if path.split(':').any(|p| p == install_dir_str) {
        return Ok(());
    }

    let home = std::env::var("HOME").map(PathBuf::from)?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        home.join(".bashrc")
    } else {
        return Err(format!(
            "unknown shell; add {} to PATH manually",
            install_dir.display()
        )
        .into());
    };

    let export_line = format!(r#"export PATH="{}:$PATH""#, install_dir_str);
    let existing = std::fs::read_to_string(&rc).unwrap_or_default();
    if !existing.contains(&export_line) {
        let mut out = existing;
        if !out.ends_with('\n') && !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&export_line);
        out.push('\n');
        std::fs::write(&rc, out)?;
        println!("Updated shell rc: {}", rc.display());
        println!("Reload: source {}", rc.display());
    }
    Ok(())
}

fn install_skill_folder(skill_name: &str) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let exe_dir = exe.parent().ok_or("invalid current exe dir")?;
    let candidate = exe_dir
        .join(".trae")
        .join("skills")
        .join(skill_name)
        .join("SKILL.md");
    if !candidate.exists() {
        return Ok(None);
    }

    let base = if cfg!(windows) {
        let home = std::env::var("USERPROFILE").map(PathBuf::from)?;
        home.join(".trae")
    } else {
        let home = std::env::var("HOME").map(PathBuf::from)?;
        home.join(".trae")
    };
    let dest_dir = base.join("skills").join(skill_name);
    std::fs::create_dir_all(&dest_dir)?;
    std::fs::copy(&candidate, dest_dir.join("SKILL.md"))?;

    let cursor_base = if cfg!(windows) {
        let home = std::env::var("USERPROFILE").map(PathBuf::from)?;
        home.join(".cursor")
    } else {
        let home = std::env::var("HOME").map(PathBuf::from)?;
        home.join(".cursor")
    };
    if cursor_base.exists() {
        let cursor_dest_dir = cursor_base.join("skills").join(skill_name);
        if std::fs::create_dir_all(&cursor_dest_dir).is_ok() {
            let _ = std::fs::copy(&candidate, cursor_dest_dir.join("SKILL.md"));
        }
    }

    Ok(Some(dest_dir))
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
        eprintln!("usage: mpa publish wechat-draft (--md <file.md> | --html <file.html>) --cover <cover.jpg> [--title <title>] [--author <name>] [--digest <text>] [--mode local|api|ai] [--theme <name>]");
        std::process::exit(2);
    }

    let sub = &args[0];
    match sub.as_str() {
        "wechat-draft" => publish_wechat_draft(args[1..].to_vec()).await,
        _ => Err(format!("unknown publish target: {sub}").into()),
    }
}

async fn publish_wechat_draft(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut md_path: Option<PathBuf> = None;
    let mut html_path: Option<PathBuf> = None;
    let mut title: Option<String> = None;
    let mut cover: Option<PathBuf> = None;
    let mut author: Option<String> = None;
    let mut digest: Option<String> = None;
    let mut mode: ConvertMode = ConvertMode::Local;
    let mut theme: String = "default".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--md" => {
                i += 1;
                md_path = Some(PathBuf::from(args.get(i).ok_or("missing --md value")?));
            }
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
            other => return Err(format!("unknown arg: {other}").into()),
        }
        i += 1;
    }

    let cfg = Config::load();
    let wechat_cfg = cfg
        .wechat
        .clone()
        .ok_or("missing WECHAT_APPID/WECHAT_SECRET (or configure in mpa tui)")?;
    let publisher_cover = WechatPublisher::new(wechat_cfg.clone())?;
    let publisher_assets = WechatPublisher::new(wechat_cfg.clone())?;
    let publisher_draft = WechatPublisher::new(wechat_cfg)?;

    let cover_path = cover.ok_or("missing --cover")?;
    let uploaded = publisher_cover.upload_image_file(&cover_path).await?;

    let (mut content_html, assets, inferred) = if let Some(md_path) = md_path {
        let markdown = std::fs::read_to_string(&md_path)?;
        let meta = mpa_core::parse_article_metadata(&markdown);

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
                markdown: markdown.clone(),
                mode,
                theme: theme.clone(),
                api_key: None,
                font_size: None,
                background_type: None,
                custom_prompt: None,
            })
            .await?;

        if result.prompt.is_some() {
            return Err("convert returned prompt; use --mode local|api for publishing".into());
        }
        let html = result.html.ok_or("no html produced")?;

        let base_dir = md_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let assets = result
            .images
            .into_iter()
            .map(|img| {
                let (kind, source, prompt) = match img.kind {
                    mpa_core::ImageKind::Local => (AssetKind::Local, img.original.clone(), None),
                    mpa_core::ImageKind::Remote => (AssetKind::Remote, img.original.clone(), None),
                    mpa_core::ImageKind::Ai => {
                        let p = img.ai_prompt.clone().or(Some(img.original.clone()));
                        (AssetKind::Ai, img.original.clone(), p)
                    }
                };
                let resolved_source = if kind == AssetKind::Local {
                    Some(base_dir.join(&source).to_string_lossy().to_string())
                } else {
                    None
                };
                AssetRef {
                    index: img.index,
                    kind,
                    source: source.clone(),
                    resolved_source,
                    prompt,
                    placeholder: Some(img.placeholder),
                    media_id: None,
                    public_url: None,
                }
            })
            .collect::<Vec<_>>();

        (html, assets, Some(meta))
    } else {
        let html_path = html_path.ok_or("missing --md or --html")?;
        let html = std::fs::read_to_string(&html_path)?;
        let base_dir = html_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let assets = parse_assets_from_html(&html, &base_dir);
        (html, assets, None)
    };

    if !assets.is_empty() {
        let processor = mpa_core::asset_processor::WechatAssetProcessor::new(cfg.clone(), publisher_assets)?;
        let pipeline = AssetPipeline::new(processor);
        let out = pipeline.process(&ProcessInput {
            html: content_html.clone(),
            assets,
        }).await?;
        content_html = out.html;
    }

    let title = title
        .or_else(|| inferred.as_ref().map(|m| m.title.clone()))
        .filter(|t| !t.trim().is_empty())
        .ok_or("missing --title (or provide title in markdown front matter)")?;
    let author = author.or_else(|| inferred.as_ref().and_then(|m| m.author.clone()));
    let digest = digest.or_else(|| inferred.as_ref().and_then(|m| m.digest.clone()));

    let result = publisher_draft
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

fn parse_assets_from_html(html: &str, base_dir: &Path) -> Vec<AssetRef> {
    let mut assets = Vec::new();
    let re_double =
        Regex::new(r#"(?i)<img[^>]*src="([^"]+)"[^>]*>"#).expect("img double regex");
    let re_single =
        Regex::new(r#"(?i)<img[^>]*src='([^']+)'[^>]*>"#).expect("img single regex");

    let mut srcs = Vec::new();
    for cap in re_double.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            srcs.push(m.as_str().to_string());
        }
    }
    for cap in re_single.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            srcs.push(m.as_str().to_string());
        }
    }

    for src in srcs.into_iter().filter(|s| !s.trim().is_empty()) {
        let index = assets.len();
        let placeholder = format!("<!-- IMG:{index} -->");

        let kind = if src.starts_with("http://") || src.starts_with("https://") {
            AssetKind::Remote
        } else if src.starts_with("__generate:") && src.ends_with("__") {
            AssetKind::Ai
        } else {
            AssetKind::Local
        };
        let prompt = if kind == AssetKind::Ai {
            Some(
                src.trim_start_matches("__generate:")
                    .trim_end_matches("__")
                    .trim()
                    .to_string(),
            )
        } else {
            None
        };

        let resolved_source = if kind == AssetKind::Local {
            Some(base_dir.join(&src).to_string_lossy().to_string())
        } else {
            None
        };

        assets.push(AssetRef {
            index,
            kind,
            source: if kind == AssetKind::Ai {
                prompt.clone().unwrap_or(src.clone())
            } else {
                src
            },
            resolved_source,
            prompt,
            placeholder: Some(placeholder),
            media_id: None,
            public_url: None,
        });
    }

    assets
}
