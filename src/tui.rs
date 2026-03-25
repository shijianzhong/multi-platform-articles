use crate::config::{Config, ConfigError};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Field {
    #[default]
    AppId,
    Secret,
    ImageProvider,
    ImageApiKey,
    ImageApiBase,
    ImageModel,
    ImageSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    WeChat,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Default)]
struct State {
    tab: Tab,
    field: Field,
    appid: String,
    secret: String,
    image_provider: String,
    image_api_key: String,
    image_api_base: String,
    image_model: String,
    image_size: String,
    message: Option<(String, MessageKind)>,
    dirty: bool,
    reveal_secret: bool,
    env_override_appid: bool,
    env_override_secret: bool,
    env_override_image_provider: bool,
    env_override_image_api_key: bool,
    env_override_image_api_base: bool,
    env_override_image_model: bool,
    env_override_image_size: bool,
}

pub fn run() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = load_state();
    let result = (|| -> Result<(), io::Error> {
        loop {
            terminal.draw(|f| draw(f, &state))?;

            if event::poll(std::time::Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                        KeyCode::Left => {
                            if state.field == Field::ImageProvider {
                                state.image_provider = cycle_provider(&state.image_provider, -1);
                                state.dirty = true;
                            } else {
                                state.tab = Tab::WeChat;
                                state.field = Field::AppId;
                            }
                        }
                        KeyCode::Right => {
                            if state.field == Field::ImageProvider {
                                state.image_provider = cycle_provider(&state.image_provider, 1);
                                state.dirty = true;
                            } else {
                                state.tab = Tab::Image;
                                state.field = Field::ImageProvider;
                            }
                        }
                        KeyCode::Tab => {
                            state.field = match state.tab {
                                Tab::WeChat => match state.field {
                                    Field::AppId => Field::Secret,
                                    _ => Field::AppId,
                                },
                                Tab::Image => match state.field {
                                    Field::ImageProvider => Field::ImageApiKey,
                                    Field::ImageApiKey => Field::ImageApiBase,
                                    Field::ImageApiBase => Field::ImageModel,
                                    Field::ImageModel => Field::ImageSize,
                                    _ => Field::ImageProvider,
                                },
                            }
                        }
                        KeyCode::Up => {
                            state.field = match state.tab {
                                Tab::WeChat => match state.field {
                                    Field::Secret => Field::AppId,
                                    _ => Field::Secret,
                                },
                                Tab::Image => match state.field {
                                    Field::ImageProvider => Field::ImageSize,
                                    Field::ImageApiKey => Field::ImageProvider,
                                    Field::ImageApiBase => Field::ImageApiKey,
                                    Field::ImageModel => Field::ImageApiBase,
                                    Field::ImageSize => Field::ImageModel,
                                    _ => Field::ImageProvider,
                                },
                            }
                        }
                        KeyCode::Down => {
                            state.field = match state.tab {
                                Tab::WeChat => match state.field {
                                    Field::AppId => Field::Secret,
                                    _ => Field::AppId,
                                },
                                Tab::Image => match state.field {
                                    Field::ImageProvider => Field::ImageApiKey,
                                    Field::ImageApiKey => Field::ImageApiBase,
                                    Field::ImageApiBase => Field::ImageModel,
                                    Field::ImageModel => Field::ImageSize,
                                    _ => Field::ImageProvider,
                                },
                            }
                        }
                        KeyCode::Char('r') => {
                            if state.field == Field::Secret {
                                state.reveal_secret = !state.reveal_secret;
                            } else {
                                let target = match state.field {
                                    Field::AppId => Some(&mut state.appid),
                                    Field::ImageApiKey => Some(&mut state.image_api_key),
                                    Field::ImageApiBase => Some(&mut state.image_api_base),
                                    Field::ImageModel => Some(&mut state.image_model),
                                    Field::ImageSize => Some(&mut state.image_size),
                                    Field::ImageProvider | Field::Secret => None,
                                };
                                if let Some(target) = target {
                                    target.push('r');
                                    state.dirty = true;
                                }
                            }
                        }
                        KeyCode::Char('s') => match save_state(&state) {
                            Ok(()) => {
                                state.dirty = false;
                                state.message = Some((
                                    "已保存到本机配置文件（环境变量会覆盖该配置）".to_string(),
                                    MessageKind::Success,
                                ));
                            }
                            Err(err) => {
                                state.message =
                                    Some((format!("保存失败：{err}"), MessageKind::Error));
                            }
                        },
                        KeyCode::Backspace => {
                            let target = match state.field {
                                Field::AppId => Some(&mut state.appid),
                                Field::Secret => Some(&mut state.secret),
                                Field::ImageApiKey => Some(&mut state.image_api_key),
                                Field::ImageApiBase => Some(&mut state.image_api_base),
                                Field::ImageModel => Some(&mut state.image_model),
                                Field::ImageSize => Some(&mut state.image_size),
                                Field::ImageProvider => None, // 不允许直接编辑
                            };
                            if let Some(target) = target {
                                target.pop();
                                state.dirty = true;
                            }
                        }
                        KeyCode::Char(c) => {
                            if c != 'r' {
                                let target = match state.field {
                                    Field::AppId => Some(&mut state.appid),
                                    Field::Secret => Some(&mut state.secret),
                                    Field::ImageApiKey => Some(&mut state.image_api_key),
                                    Field::ImageApiBase => Some(&mut state.image_api_base),
                                    Field::ImageModel => Some(&mut state.image_model),
                                    Field::ImageSize => Some(&mut state.image_size),
                                    Field::ImageProvider => None, // 不允许直接编辑
                                };
                                if let Some(target) = target {
                                    target.push(c);
                                    state.dirty = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    })();

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    result
}

fn cycle_provider(current: &str, dir: i32) -> String {
    let providers = ["openai", "modelscope", "tuzi", "openrouter", "gemini"];
    let current_idx = providers
        .iter()
        .position(|&p| p == current)
        .unwrap_or(0) as i32;
    let next_idx = (current_idx + dir).rem_euclid(providers.len() as i32) as usize;
    providers[next_idx].to_string()
}

fn load_state() -> State {
    let cfg = Config::load();
    let mut state = State {
        tab: Tab::WeChat,
        field: Field::AppId,
        ..Default::default()
    };
    if let Some(w) = cfg.wechat {
        state.appid = w.appid;
        state.secret = w.secret;
    }
    if let Some(p) = cfg.image.provider {
        state.image_provider = p;
    } else {
        state.image_provider = "openai".to_string(); // 默认值
    }
    if let Some(k) = cfg.image.api_key {
        state.image_api_key = k;
    }
    if let Some(v) = cfg.image.api_base {
        state.image_api_base = v;
    }
    if let Some(v) = cfg.image.model {
        state.image_model = v;
    }
    if let Some(v) = cfg.image.size {
        state.image_size = v;
    }
    state.env_override_appid = std::env::var("WECHAT_APPID")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_secret = std::env::var("WECHAT_SECRET")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_image_provider = std::env::var("IMAGE_PROVIDER")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_image_api_key = std::env::var("IMAGE_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_image_api_base = std::env::var("IMAGE_API_BASE")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_image_model = std::env::var("IMAGE_MODEL")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_image_size = std::env::var("IMAGE_SIZE")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state
}

fn save_state(state: &State) -> Result<(), ConfigError> {
    let cfg = Config::load();
    cfg.save_credentials(
        state.appid.clone(),
        state.secret.clone(),
        state.image_provider.clone(),
        state.image_api_key.clone(),
        state.image_api_base.clone(),
        state.image_model.clone(),
        state.image_size.clone(),
    )
}

fn draw(f: &mut Frame, state: &State) {
    let area = f.area();
    f.render_widget(Clear, area);

    let width = area.width.min(92);
    let height = area.height.min(36);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let card = Rect {
        x,
        y,
        width,
        height,
    };

    let border = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(border, card);

    let inner = Rect {
        x: card.x + 1,
        y: card.y + 1,
        width: card.width.saturating_sub(2),
        height: card.height.saturating_sub(2),
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(12),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Multi-Platform Articles",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default());
    f.render_widget(header, layout[0]);

    let tabs = Tabs::new(vec![Line::from("WeChat"), Line::from("Image")])
        .select(if state.tab == Tab::WeChat { 0 } else { 1 })
        .highlight_style(Style::default().fg(Color::Cyan))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(tabs, layout[1]);

    let active_border = Style::default().fg(Color::Cyan);
    let inactive_border = Style::default().fg(Color::DarkGray);
    let dim = Style::default().fg(Color::DarkGray);

    let mut fields: Vec<(String, Text, Field)> = Vec::new();
    match state.tab {
        Tab::WeChat => {
            let title = if state.env_override_appid {
                "WECHAT_APPID (env 覆盖中)".to_string()
            } else {
                "WECHAT_APPID".to_string()
            };
            let text = if state.appid.is_empty() {
                Text::from(Line::styled("在此输入 AppID…", dim))
            } else {
                Text::from(state.appid.as_str())
            };
            fields.push((title, text, Field::AppId));

            let title = if state.env_override_secret {
                "WECHAT_SECRET (env 覆盖中)".to_string()
            } else {
                "WECHAT_SECRET".to_string()
            };
            let text = if state.secret.is_empty() {
                Text::from(Line::styled("在此输入 Secret…", dim))
            } else if state.reveal_secret {
                Text::from(state.secret.as_str())
            } else {
                Text::from("•".repeat(state.secret.chars().count()))
            };
            fields.push((format!("{title}  (r 显示/隐藏)"), text, Field::Secret));
        }
        Tab::Image => {
            let title = if state.env_override_image_provider {
                "IMAGE_PROVIDER (env 覆盖中)".to_string()
            } else {
                "IMAGE_PROVIDER (openai/tuzi/modelscope/openrouter/gemini)".to_string()
            };
            let text = if state.image_provider.is_empty() {
                Text::from(Line::styled("使用左右方向键切换服务商...", dim))
            } else {
                Text::from(Line::from(vec![
                    Span::styled("◄ ", dim),
                    Span::styled(state.image_provider.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(" ►", dim),
                ]))
            };
            fields.push((title, text, Field::ImageProvider));

            let title = if state.env_override_image_api_key {
                "IMAGE_API_KEY (env 覆盖中)".to_string()
            } else {
                "IMAGE_API_KEY".to_string()
            };
            let text = if state.image_api_key.is_empty() {
                Text::from(Line::styled("在此输入配图服务 API Key…", dim))
            } else {
                Text::from(state.image_api_key.as_str())
            };
            fields.push((title, text, Field::ImageApiKey));

            let title = if state.env_override_image_api_base {
                "IMAGE_API_BASE (env 覆盖中)".to_string()
            } else {
                "IMAGE_API_BASE (可选，tuzi 必填)".to_string()
            };
            let text = if state.image_api_base.is_empty() {
                Text::from(Line::styled("在此输入 API Base…", dim))
            } else {
                Text::from(state.image_api_base.as_str())
            };
            fields.push((title, text, Field::ImageApiBase));

            let title = if state.env_override_image_model {
                "IMAGE_MODEL (env 覆盖中)".to_string()
            } else {
                "IMAGE_MODEL (可选)".to_string()
            };
            let text = if state.image_model.is_empty() {
                Text::from(Line::styled("在此输入模型…", dim))
            } else {
                Text::from(state.image_model.as_str())
            };
            fields.push((title, text, Field::ImageModel));

            let title = if state.env_override_image_size {
                "IMAGE_SIZE (env 覆盖中)".to_string()
            } else {
                "IMAGE_SIZE (可选，如 1024x1024)".to_string()
            };
            let text = if state.image_size.is_empty() {
                Text::from(Line::styled("在此输入尺寸…", dim))
            } else {
                Text::from(state.image_size.as_str())
            };
            fields.push((title, text, Field::ImageSize));
        }
    }

    let constraints: Vec<Constraint> = std::iter::repeat(Constraint::Length(4))
        .take(fields.len())
        .collect();
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(layout[2]);
    for (i, (title, text, field)) in fields.into_iter().enumerate() {
        let border = if state.field == field {
            active_border
        } else {
            inactive_border
        };
        let mut p = Paragraph::new(text)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border),
            )
            .style(Style::default().fg(Color::White));
        if field == Field::ImageProvider {
            p = p.alignment(Alignment::Center);
        }
        f.render_widget(p, body[i]);
    }

    let hints = vec![
        Span::styled("Tab/↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" 切换  "),
        Span::styled("←/→", Style::default().fg(Color::Cyan)),
        Span::raw(" 分页  "),
        Span::styled("输入/Backspace", Style::default().fg(Color::Cyan)),
        Span::raw(" 编辑  "),
        Span::styled("s", Style::default().fg(Color::Cyan)),
        Span::raw(" 保存  "),
        Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" 退出"),
    ];

    let mut info_lines = Vec::<Line>::new();
    if state.dirty {
        info_lines.push(Line::from(vec![
            Span::styled("●", Style::default().fg(Color::Yellow)),
            Span::raw(" 未保存更改"),
        ]));
    } else {
        info_lines.push(Line::from(vec![
            Span::styled("●", Style::default().fg(Color::Green)),
            Span::raw(" 已保存"),
        ]));
    }
    info_lines.push(Line::from(hints));

    let help = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(inactive_border)
            .title("快捷键"),
    );
    f.render_widget(help, layout[3]);

    let config_path = Config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "-".to_string());
    let env_note = if state.env_override_appid
        || state.env_override_secret
        || state.env_override_image_provider
        || state.env_override_image_api_key
        || state.env_override_image_api_base
        || state.env_override_image_model
        || state.env_override_image_size
    {
        "环境变量将覆盖配置"
    } else {
        "可用环境变量覆盖"
    };

    let (msg, kind) = state.message.clone().unwrap_or_else(|| {
        (
            format!("配置文件：{config_path}  ·  {env_note}"),
            MessageKind::Info,
        )
    });
    let msg_style = match kind {
        MessageKind::Info => Style::default().fg(Color::DarkGray),
        MessageKind::Success => Style::default().fg(Color::Green),
        MessageKind::Error => Style::default().fg(Color::Red),
    };
    let status = Paragraph::new(msg)
        .style(msg_style)
        .alignment(Alignment::Left)
        .block(Block::default());
    f.render_widget(status, layout[4]);
}
