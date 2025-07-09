use std::sync::Arc;

use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::Clear,
    {Terminal, TerminalOptions, Viewport},
};

use russh::ChannelId;

use russh::server::{Handle, Session};

use emojic::text::parse_text;

use crate::{
    app::{AppState, SshTerminal, TerminalHandle, app_state::InputMode},
    app_server::AppServerController,
    db_models::{Message, User},
};

pub struct AppController {
    server_controller: Arc<AppServerController>,
    pub terminal: SshTerminal,
    pub app_state: AppState,
    handle: Handle,
    channel_id: ChannelId,
    pub active: bool,
}

impl AppController {
    pub async fn new(
        server_controller: Arc<AppServerController>,
        session: &mut Session,
        channel_id: ChannelId,
        user: User,
    ) -> Result<Self, anyhow::Error> {
        let terminal_handle = TerminalHandle::start(session.handle(), channel_id).await;

        let backend = CrosstermBackend::new(terminal_handle);

        let options = TerminalOptions {
            viewport: Viewport::Fixed(Rect::default()),
        };

        let terminal = Terminal::with_options(backend, options)?;
        Ok(Self {
            server_controller,
            terminal,
            app_state: AppState::new(user),
            handle: session.handle(),
            channel_id: channel_id,
            active: true,
        })
    }

    pub fn resize_terminal(&mut self, rect: Rect) {
        let _ = self.terminal.resize(rect);
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let messages: Vec<Message> = self.server_controller.get_messages().await?;
        Ok(messages)
    }

    pub async fn send_message(&self, message: String) -> Result<(), anyhow::Error> {
        self.server_controller
            .send_message(Message::new(message, self.app_state.user.clone()))
            .await
    }

    pub fn write_to_input(&mut self, char: Option<char>) {
        if let Some(char) = char {
            self.app_state.input_message.push(char);
        } else {
            self.app_state.input_message.pop();
        }
        self.app_state.input_message = parse_text(&self.app_state.input_message);
    }

    pub fn clear_input(&mut self) {
        self.app_state.input_message.clear();
    }

    pub fn get_input_message(&self) -> String {
        self.app_state.input_message.clone()
    }

    pub async fn draw(&mut self) -> Result<(), anyhow::Error> {
        self.app_state.messages = self.get_messages().await?;
        self.app_state.users = self.get_users().await;

        self.terminal.draw(|frame| {
            frame.render_widget(Clear, frame.area());
            frame.render_widget(&mut self.app_state, frame.area());
        })?;
        Ok(())
    }

    pub async fn get_users(&self) -> Vec<User> {
        self.server_controller.get_users().await
    }

    pub async fn disconnect(&mut self) {
        if self.handle.close(self.channel_id).await.is_ok() {
            self.active = false;
        }
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.app_state.input_mode = mode;
    }

    pub fn scroll_up(&mut self, count: u16) {
        if (self.app_state.scroll_offset) >= count {
            self.app_state.scroll_offset = self.app_state.scroll_offset - count;
        } else {
            self.app_state.scroll_offset = 0;
        }
    }

    pub fn scroll_down(&mut self, count: u16) {
        self.app_state.scroll_offset = self.app_state.scroll_offset + count;
    }
}
