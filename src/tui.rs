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

#[derive(Debug, Default)]
struct State {
    field: Field,
    appid: String,
    secret: String,
    message: Option<String>,
    dirty: bool,
    reveal_secret: bool,
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
                                state.message = Some(
                                    "已保存到本机配置文件（环境变量会覆盖该配置）".to_string(),
                                );
                            }
                            Err(err) => {
                                state.message = Some(format!("保存失败：{err}"));
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
    state
}

fn save_state(state: &State) -> Result<(), ConfigError> {
    let cfg = Config::load();
    cfg.save_wechat_credentials(state.appid.clone(), state.secret.clone())
}

fn draw(f: &mut Frame, state: &State) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let title = Paragraph::new("mpa 配置（微信公众号）")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let input_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
        .split(chunks[1]);

    let appid_style = if state.field == Field::AppId {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let secret_style = if state.field == Field::Secret {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let appid = Paragraph::new(state.appid.as_str()).block(
        Block::default()
            .title("WECHAT_APPID")
            .borders(Borders::ALL)
            .border_style(appid_style),
    );
    f.render_widget(appid, input_rows[0]);

    let secret_value = if state.reveal_secret {
        state.secret.clone()
    } else {
        "*".repeat(state.secret.chars().count())
    };
    let secret = Paragraph::new(secret_value).block(
        Block::default()
            .title("WECHAT_SECRET (按 r 显示/隐藏)")
            .borders(Borders::ALL)
            .border_style(secret_style),
    );
    f.render_widget(secret, input_rows[1]);

    let mut help = Vec::<Line>::new();
    help.push(Line::from("Tab/↑↓ 切换字段  输入/Backspace 编辑"));
    help.push(Line::from("s 保存  r 显示/隐藏 secret  q/Esc 退出"));
    if state.dirty {
        help.push(Line::from("未保存更改").style(Style::default().fg(Color::Yellow)));
    }
    let help = Paragraph::new(help).block(Block::default().borders(Borders::ALL).title("操作"));
    f.render_widget(help, chunks[2]);

    let msg = state.message.clone().unwrap_or_else(|| {
        format!(
            "配置文件：{}",
            Config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "-".to_string())
        )
    });
    let msg = Paragraph::new(msg).block(Block::default().borders(Borders::ALL).title("提示"));
    f.render_widget(msg, chunks[3]);
}
