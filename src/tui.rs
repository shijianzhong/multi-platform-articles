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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Default)]
struct State {
    field: Field,
    appid: String,
    secret: String,
    message: Option<(String, MessageKind)>,
    dirty: bool,
    reveal_secret: bool,
    env_override_appid: bool,
    env_override_secret: bool,
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
                        KeyCode::Tab => {
                            state.field = match state.field {
                                Field::AppId => Field::Secret,
                                Field::Secret => Field::AppId,
                            };
                        }
                        KeyCode::Up => state.field = Field::AppId,
                        KeyCode::Down => state.field = Field::Secret,
                        KeyCode::Char('r') => {
                            state.reveal_secret = !state.reveal_secret;
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
                                Field::AppId => &mut state.appid,
                                Field::Secret => &mut state.secret,
                            };
                            target.pop();
                            state.dirty = true;
                        }
                        KeyCode::Char(c) => {
                            let target = match state.field {
                                Field::AppId => &mut state.appid,
                                Field::Secret => &mut state.secret,
                            };
                            target.push(c);
                            state.dirty = true;
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

fn load_state() -> State {
    let cfg = Config::load();
    let mut state = State {
        field: Field::AppId,
        ..Default::default()
    };
    if let Some(w) = cfg.wechat {
        state.appid = w.appid;
        state.secret = w.secret;
    }
    state.env_override_appid = std::env::var("WECHAT_APPID")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state.env_override_secret = std::env::var("WECHAT_SECRET")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    state
}

fn save_state(state: &State) -> Result<(), ConfigError> {
    let cfg = Config::load();
    cfg.save_wechat_credentials(state.appid.clone(), state.secret.clone())
}

fn draw(f: &mut Frame, state: &State) {
    let area = f.area();
    f.render_widget(Clear, area);

    let width = area.width.min(92);
    let height = area.height.min(22);
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
            Constraint::Length(8),
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

    let tabs = Tabs::new(vec![Line::from("WeChat Credentials")])
        .select(0)
        .highlight_style(Style::default().fg(Color::Cyan))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(tabs, layout[1]);

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Length(4)].as_ref())
        .split(layout[2]);

    let active_border = Style::default().fg(Color::Cyan);
    let inactive_border = Style::default().fg(Color::DarkGray);
    let dim = Style::default().fg(Color::DarkGray);

    let appid_title = if state.env_override_appid {
        "WECHAT_APPID (env 覆盖中)"
    } else {
        "WECHAT_APPID"
    };
    let appid_border = if state.field == Field::AppId {
        active_border
    } else {
        inactive_border
    };
    let appid_text = if state.appid.is_empty() {
        Text::from(Line::styled("在此输入 AppID…", dim))
    } else {
        Text::from(state.appid.as_str())
    };
    let appid = Paragraph::new(appid_text)
        .block(
            Block::default()
                .title(appid_title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(appid_border),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(appid, body[0]);

    let secret_title = if state.env_override_secret {
        "WECHAT_SECRET (env 覆盖中)"
    } else {
        "WECHAT_SECRET"
    };
    let secret_border = if state.field == Field::Secret {
        active_border
    } else {
        inactive_border
    };
    let secret_value = if state.secret.is_empty() {
        Text::from(Line::styled("在此输入 Secret…", dim))
    } else if state.reveal_secret {
        Text::from(state.secret.as_str())
    } else {
        Text::from("•".repeat(state.secret.chars().count()))
    };
    let secret = Paragraph::new(secret_value)
        .block(
            Block::default()
                .title(format!("{secret_title}  (r 显示/隐藏)"))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(secret_border),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(secret, body[1]);

    let hints = vec![
        Span::styled("Tab/↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" 切换  "),
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
    let env_note = if state.env_override_appid || state.env_override_secret {
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
