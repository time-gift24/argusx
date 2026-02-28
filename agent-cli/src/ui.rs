use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;

pub fn draw(frame: &mut Frame<'_>, app: &AppState) {
    // Paint a full-screen background first to avoid terminal transparency bleed-through.
    let panel_style = Style::default()
        .bg(Color::Rgb(16, 18, 22))
        .fg(Color::Rgb(230, 230, 230));
    frame.render_widget(Block::default().style(panel_style), frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Build history lines from messages
    let mut history_lines: Vec<String> = app.messages.iter().map(|m| m.text.clone()).collect();

    // Add reasoning text if show_reasoning is enabled and there's reasoning content
    if app.show_reasoning && !app.reasoning_text.is_empty() {
        history_lines.push(format!("[reasoning] {}", app.reasoning_text));
    }

    // Add tool progress items
    for item in &app.tool_progress {
        history_lines.push(format!("[tool:{}] {}", item.tool_name, item.status));
    }

    let history_text = history_lines.join("\n");

    let history = Paragraph::new(history_text)
        .style(panel_style)
        .block(
            Block::default()
                .title("Chat")
                .borders(Borders::ALL)
                .style(panel_style),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(history, chunks[0]);

    let status_text = if let Some(warning) = app.last_warning.as_deref() {
        format!("status: {warning}")
    } else if app.active_turn.is_some() {
        "status: running".to_string()
    } else {
        format!("status: ready (session: {})", app.session_id)
    };
    frame.render_widget(Paragraph::new(status_text).style(panel_style), chunks[1]);

    let input = Paragraph::new(app.input.clone()).style(panel_style).block(
        Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .style(panel_style),
    );
    frame.render_widget(input, chunks[2]);
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

    #[test]
    fn folded_reasoning_hides_reasoning_lines() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new("s-1".into());
        app.show_reasoning = false;
        app.reasoning_text = "secret reasoning".into();

        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let content = buffer_to_string(buf);
        assert!(
            !content.contains("secret reasoning"),
            "buffer should NOT contain reasoning when folded, got: {}",
            content
        );
    }

    #[test]
    fn tool_progress_renders_status_labels() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new("s-1".into());
        app.tool_progress.push(crate::app::ToolProgressItem {
            call_id: "call-1".into(),
            tool_name: "read_file".into(),
            status: "running".into(),
        });

        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let content = buffer_to_string(buf);
        assert!(
            content.contains("read_file"),
            "buffer should contain tool_name, got: {}",
            content
        );
        assert!(
            content.contains("running"),
            "buffer should contain status, got: {}",
            content
        );
    }

    #[test]
    fn warning_status_is_visible_when_present() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = AppState::new("s-1".into());
        app.last_warning = Some("error: chat error: timeout".into());

        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let content = buffer_to_string(buf);
        assert!(
            content.contains("timeout"),
            "buffer should contain warning text, got: {}",
            content
        );
    }

    #[test]
    fn render_sets_non_reset_background() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = AppState::new("s-1".into());

        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let first = &buf.content[0];
        assert_ne!(first.bg, Color::Reset, "background color should be set");
    }
}
