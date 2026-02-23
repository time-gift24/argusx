use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;

pub fn draw(frame: &mut Frame<'_>, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let history_text = app
        .messages
        .iter()
        .map(|m| m.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let history = Paragraph::new(history_text)
        .block(Block::default().title("Chat").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(history, chunks[0]);

    let input = Paragraph::new(app.input.clone())
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let mut lines = Vec::new();
        for y in 0..buf.area.height {
            let mut line = String::new();
            for x in 0..buf.area.width {
                let cell = &buf.content[(y * buf.area.width + x) as usize];
                if !cell.skip {
                    line.push_str(cell.symbol());
                }
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    #[test]
    fn render_shows_input_and_messages() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new("s-1".into());
        app.input = "hello".into();
        app.messages.push(crate::app::MessageItem {
            role: crate::app::Role::Assistant,
            text: "hi".into(),
        });

        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let content = buffer_to_string(buf);

        assert!(
            content.contains("hi"),
            "buffer should contain assistant message 'hi', got: {}",
            content
        );
        assert!(
            content.contains("hello"),
            "buffer should contain input 'hello', got: {}",
            content
        );
    }
}
