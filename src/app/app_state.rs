use ratatui::{
    buffer::Buffer,
    layout::Rect,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::db_models::{Message, User};

pub enum InputMode {
    Insert,
    Navigate,
}

pub struct AppState {
    pub input_message: String,
    pub messages: Vec<Message>,
    pub user: User,
    pub users: Vec<User>,
    pub input_mode: InputMode,
    pub scroll_offset: u16,
}

impl AppState {
    pub fn new(user: User) -> Self {
        Self {
            input_message: String::new(),
            messages: Vec::new(),
            user: user,
            users: Vec::new(),
            input_mode: InputMode::Insert,
            scroll_offset: 0,
        }
    }
}

impl Widget for &mut AppState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_area = Rect::new(0, 0, area.width, area.height.saturating_sub(4));
        let input_area = Rect::new(0, area.height.saturating_sub(4), area.width, 3);
        let help_area = Rect::new(0, area.height.saturating_sub(1), area.width, 1);

        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(main_area);

        let message_area = main_layout[0];
        let users_area = main_layout[1];

        self.scroll_offset = self
            .scroll_offset
            .min((self.messages.len() as isize - message_area.height as isize + 2).max(0) as u16);

        let message_list: Vec<Line> = self
            .messages
            .iter()
            .map(|message| {
                Line::raw(format!(
                    "{sender}: {message}",
                    message = message.content,
                    sender = message.sender.username
                ))
            })
            .rev()
            .skip(self.scroll_offset as usize)
            .take(message_area.height.saturating_sub(2) as usize)
            .rev()
            .collect();

        let user_list: Vec<Line> = self
            .users
            .iter()
            .map(|user| Line::raw(format!("@{}", user.username)))
            .collect();

        Paragraph::new(message_list)
            .block(Block::new().borders(Borders::ALL))
            .render(message_area, buf);

        Paragraph::new(user_list)
            .block(Block::new().borders(Borders::ALL))
            .render(users_area, buf);

        if let InputMode::Insert = self.input_mode {
            Paragraph::new(format!("> {}▉", self.input_message))
                .block(Block::new().borders(Borders::ALL))
                .render(input_area, buf);
        }

        match self.input_mode {
            InputMode::Insert => {
                Paragraph::new("Ctrl-N: navigate mode | Ctrl-Q: exit").render(help_area, buf);
            }
            InputMode::Navigate => {
                Paragraph::new(format!("Enter: exit navigate mode | k: scroll up | j: scroll down | q: exit | offset: {}", self.scroll_offset)).render(help_area, buf);
            }
        }
    }
}
