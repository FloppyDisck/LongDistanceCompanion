use dotenv_codegen::dotenv;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::prelude::{
    Color, Constraint, CrosstermBackend, Direction, Layout, Line, Style, Stylize, Text,
};
use ratatui::symbols::border;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Terminal};
use reqwest::{Client, Response, Url};
use secp256k1::SecretKey;
use serde::Serialize;
use server::{sign, Active, Message, Tick, TickType, TriggerTick};
use std::io;
use std::str::FromStr;
//
// #[tokio::main]
// async fn main() -> io::Result<()> {
//     let url = Url::parse(dotenv!("CLIENT_URL")).unwrap();
//     let priv_key = SecretKey::from_str(dotenv!("SECRET_KEY")).unwrap();
//
//     let mut terminal = ratatui::init();
//     terminal.clear()?;
//     let state = State::new(url, priv_key).await;
//     let app_result = run(terminal, state).await;
//     ratatui::restore();
//     app_result
//
//     // dbg!(get_message(&url).await);
//     // dbg!(get_sequence(&url).await);
//     // dbg!(get_active(&url).await);
//     // dbg!(get_ticks(&url).await);
//     //
//     // set_message(
//     //     &url,
//     //     &priv_key,
//     //     format!("message {}", get_sequence(&url).await),
//     // )
//     // .await;
//     // set_active(&url, &priv_key, !get_active(&url).await).await;
//     // tick(&url, &priv_key, 1).await;
//     // tick(&url, &priv_key, 2).await;
//     // tick(&url, &priv_key, 3).await;
//     //
//     // dbg!(get_tick_history(&url).await);
//     // dbg!(get_message(&url).await);
//     // dbg!(get_sequence(&url).await);
//     // dbg!(get_active(&url).await);
// }

async fn get_sequence(url: &Url) -> u64 {
    reqwest::get(url.join("/sequence").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
        .parse()
        .unwrap()
}

async fn get_message(url: &Url) -> String {
    reqwest::get(url.join("/message").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

async fn get_active(url: &Url) -> bool {
    reqwest::get(url.join("/active").unwrap())
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
        .parse()
        .unwrap()
}

async fn post<T: Serialize>(url: &Url, path: &str, privkey: &SecretKey, message: T) -> Response {
    let sequence = get_sequence(&url).await;
    Client::builder()
        .build()
        .unwrap()
        .post(url.join(path).unwrap())
        .json(&message)
        .header("auth", sign(privkey, message, sequence).to_string())
        .send()
        .await
        .unwrap()
}

async fn set_message(url: &Url, privkey: &SecretKey, message: String) {
    post(url, "/message", privkey, Message { message }).await;
}

async fn set_active(url: &Url, privkey: &SecretKey, active: bool) {
    post(url, "/active", privkey, Active { active }).await;
}

async fn get_ticks(url: &Url) -> Vec<TickType> {
    reqwest::get(url.join("/ticks").unwrap())
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn get_tick_history(url: &Url) -> Vec<Tick> {
    reqwest::get(url.join("/tick_history").unwrap())
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn tick(url: &Url, privkey: &SecretKey, tick: u8) {
    post(url, "/tick", privkey, TriggerTick { ty: tick }).await;
}

async fn healthy(url: &Url) -> bool {
    reqwest::get(url.join("/").unwrap())
        .await
        .unwrap()
        .status()
        .is_success()
}

enum SelectedWindow {
    Text,
    Tick,
    TickHistory,
}

struct App {
    url: Url,
    priv_key: SecretKey,
    server_message: String,
    ticks: Vec<TickType>,
    status: bool,

    selected: SelectedWindow,

    local_message: String,
    selected_action: usize,

    scroll_offset: usize,
    tick_history: Vec<String>,
}

fn tick_to_string(ticks: &Vec<TickType>, tick_history: Vec<Tick>) -> Vec<String> {
    tick_history
        .iter()
        .map(|t| {
            format!(
                "{} - {}",
                ticks
                    .iter()
                    .find_map(|tick| if t.tick == tick.id {
                        Some(tick.tick.clone())
                    } else {
                        None
                    })
                    .unwrap(),
                t.time
            )
        })
        .rev()
        .collect()
}

impl App {
    async fn new(url: Url, priv_key: SecretKey) -> App {
        let status = healthy(&url).await;
        let server_message = get_message(&url).await;
        let ticks = get_ticks(&url).await;
        let tick_history = tick_to_string(&ticks, get_tick_history(&url).await);

        App {
            url,
            priv_key,
            ticks,
            tick_history,
            status,
            server_message,
            local_message: String::new(),
            selected_action: 0,
            selected: SelectedWindow::Text,
            scroll_offset: 0,
        }
    }

    pub async fn reload(&mut self) {
        self.status = healthy(&self.url).await;
        self.ticks = get_ticks(&self.url).await;
        self.local_message.clear();
        self.server_message = get_message(&self.url).await;
        self.tick_history = tick_to_string(&self.ticks, get_tick_history(&self.url).await);
        self.scroll_offset = 0;
    }

    fn next_mode(&mut self) {
        self.selected = match self.selected {
            SelectedWindow::Text => SelectedWindow::Tick,
            SelectedWindow::Tick => SelectedWindow::TickHistory,
            SelectedWindow::TickHistory => SelectedWindow::Text,
        }
    }

    async fn handle_input(&mut self, key: KeyCode) {
        match self.selected {
            SelectedWindow::Text => match key {
                KeyCode::Enter => {
                    set_message(&self.url, &self.priv_key, self.local_message.clone()).await;
                    self.reload().await;
                }
                KeyCode::Char(c) => self.local_message.push(c),
                KeyCode::Backspace => {
                    self.local_message.pop();
                }
                KeyCode::Tab => self.next_mode(),
                _ => {}
            },
            SelectedWindow::Tick => match key {
                KeyCode::Up => {
                    self.selected_action = self.selected_action.saturating_sub(1);
                }
                KeyCode::Down => {
                    if self.selected_action < self.ticks.len() - 1 {
                        self.selected_action += 1;
                    }
                }
                KeyCode::Tab => self.next_mode(),
                KeyCode::Enter => {
                    tick(&self.url, &self.priv_key, self.selected_action as u8 + 1).await;
                    self.reload().await;
                }
                _ => {}
            },
            SelectedWindow::TickHistory => match key {
                KeyCode::Up => {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.scroll_offset < self.tick_history.len().saturating_sub(1) {
                        self.scroll_offset += 1;
                    }
                }
                KeyCode::Tab => self.next_mode(),
                _ => {}
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let url = Url::parse(dotenv!("CLIENT_URL")).unwrap();
    let priv_key = SecretKey::from_str(dotenv!("SECRET_KEY")).unwrap();

    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(url, priv_key).await;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(3),
                ])
                .split(frame.size());

            // Status display
            let status_text = format!("Status: {}", if app.status { "Ok" } else { "Error" });
            let status = Paragraph::new(status_text)
                .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(status, chunks[0]);

            // Server message
            let server_message = Paragraph::new(app.server_message.as_str())
                .style(Style::default())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Server Message"),
                );
            frame.render_widget(server_message, chunks[1]);

            let selected_style = Style::default().fg(Color::Yellow);
            let style = Style::default();

            // Local message input
            let input = Paragraph::new(app.local_message.as_str())
                .style(if matches!(app.selected, SelectedWindow::Text) {
                    selected_style.clone()
                } else {
                    style.clone()
                })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Local Message"),
                );
            frame.render_widget(input, chunks[2]);

            // Split bottom section into two columns
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[3]);

            // Action selection
            let items: Vec<ListItem> = app
                .ticks
                .iter()
                .map(|action| ListItem::new(Text::from(action.tick.clone())))
                .collect();
            let actions = List::new(items)
                .style(if matches!(app.selected, SelectedWindow::Tick) {
                    selected_style.clone()
                } else {
                    style.clone()
                })
                .block(Block::default().borders(Borders::ALL).title("Ticks"))
                .highlight_style(Style::default().fg(Color::Yellow))
                .highlight_symbol("> ");
            frame.render_stateful_widget(
                actions,
                bottom_chunks[0],
                &mut ratatui::widgets::ListState::default()
                    .with_selected(Some(app.selected_action)),
            );

            let items: Vec<ListItem> = app
                .tick_history
                .iter()
                .map(|item| ListItem::new(Text::from(item.clone())))
                .collect();
            let items_list = List::new(items)
                .style(if matches!(app.selected, SelectedWindow::TickHistory) {
                    selected_style.clone()
                } else {
                    style.clone()
                })
                .block(Block::default().borders(Borders::ALL).title("Tick History"))
                .highlight_style(Style::default().fg(Color::Yellow))
                .highlight_symbol("> ");
            frame.render_stateful_widget(
                items_list,
                bottom_chunks[1],
                &mut ratatui::widgets::ListState::default().with_selected(Some(app.scroll_offset)),
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') && !matches!(app.selected, SelectedWindow::Text) {
                break;
            }
            app.handle_input(key.code).await;
        }
    }

    disable_raw_mode()?;
    terminal.clear()?;
    Ok(())
}
