use mpa_core::converter::{ConvertMode, ConvertRequest, MarkdownConverter};
use mpa_core::platforms::wechat::WechatPublisher;
use mpa_core::platforms::{DraftArticle, Publisher};
use mpa_core::{tui, ApiConfig, Config, ThemeManager};
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
            std::process::exit(2);
        }
    }
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
            Ok(Some(path)) => println!("Installed skill: {}", path.display()),
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
