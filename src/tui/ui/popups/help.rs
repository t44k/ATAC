use ratatui::layout::{Margin, Position, Rect};
use ratatui::prelude::{Line, Span};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::app::App;
use crate::app::files::key_bindings::KEY_BINDINGS;
use crate::app::files::theme::THEME;
use crate::tui::app_states::EMPTY_KEY;
use crate::tui::mouse::MouseTarget;

/// Width of the keys column
const KEYS_WIDTH: usize = 24;

impl App<'_> {
    pub fn render_help_popup(&mut self, frame: &mut Frame) {
        let screen = frame.area();
        let width = 90.min(screen.width.saturating_sub(4));
        let height = 32.min(screen.height.saturating_sub(2));
        let area = Rect::new(screen.x + (screen.width - width) / 2, screen.y + (screen.height - height) / 2, width, height);

        let popup_block = Block::default()
            .title(format!(" Help: {} ", self.help_popup.selection))
            .title_bottom(Line::from(" ↑ ↓ or mouse wheel to scroll, Esc to close ").right_aligned())
            .borders(Borders::ALL)
            .fg(THEME.read().ui.main_foreground_color)
            .bg(THEME.read().ui.secondary_background_color);

        frame.set_cursor_position(Position::new(0, 0));
        frame.render_widget(Clear, area);
        frame.render_widget(popup_block, area);
        self.hit(area, MouseTarget::Help);

        let lines = self.help_lines();
        let content_area = area.inner(Margin::new(2, 1));

        let max_scroll = lines.len().saturating_sub(content_area.height as usize);
        self.help_scroll = self.help_scroll.min(max_scroll);

        let help_paragraph = Paragraph::new(lines).scroll((self.help_scroll as u16, 0));
        frame.render_widget(help_paragraph, content_area);

        if max_scroll > 0 {
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(self.help_scroll);
            frame.render_stateful_widget(Scrollbar::new(ScrollbarOrientation::VerticalRight), area.inner(Margin::new(0, 1)), &mut scrollbar_state);
        }
    }

    fn help_lines(&self) -> Vec<Line<'static>> {
        let key_color = THEME.read().others.selection_highlight_color;
        let title_color = THEME.read().ui.font_color;
        let text_color = THEME.read().ui.font_color;

        let section = |title: &str| Line::from(Span::raw(title.to_string()).bold().underlined().fg(title_color));
        let entry = |keys: String, description: String| Line::from(vec![
            Span::raw(format!("{keys:KEYS_WIDTH$}")).fg(key_color),
            Span::raw(description).fg(text_color),
        ]);

        let mut lines = vec![section("Keys")];

        let is_there_any_env = self.get_selected_env_as_local().is_some();
        let protocol = self.collections_tree.selected.map(|(collection, request)| self.collections[collection].requests[request].read().protocol.clone());
        let events = self.help_popup.selection.get_available_events(self.request_view, self.request_param_tab, protocol, is_there_any_env);

        for event in &events {
            let key_bindings = event.get_event_key_bindings();

            // Catch-all events of text inputs
            if key_bindings.keys.is_empty() {
                continue;
            }

            let keys = key_bindings.keys
                .iter()
                .filter(|key| **key != *EMPTY_KEY)
                .map(|key| key.to_string())
                .collect::<Vec<String>>()
                .join(", ");

            lines.push(entry(keys, key_bindings.event_name.clone()));
        }

        let key_bindings = *KEY_BINDINGS.read();

        lines.push(Line::default());
        lines.push(section("Everywhere"));
        lines.push(entry(format!("{}", key_bindings.generic.copy), String::from("Copy the selected text, or the link under the mouse")));
        lines.push(entry(format!("{}, {}", key_bindings.generic.display_help, key_bindings.generic.alt_display_help), String::from("Show this help")));
        lines.push(entry(format!("{}, {}", key_bindings.main_menu.exit, key_bindings.main_menu.alt_exit), String::from("Quit (from the collections)")));

        lines.push(Line::default());
        lines.push(section("Mouse"));
        lines.push(entry(String::from("Click"), String::from("Open a request, switch tab, edit a field, select a table cell")));
        lines.push(entry(String::from("Click selected cell"), String::from("Edit the table cell (double click works too)")));
        lines.push(entry(String::from("Click elsewhere"), String::from("Save the field being edited, or close the popup")));
        lines.push(entry(String::from("Wheel"), String::from("Scroll the panel under the mouse (Shift + wheel: horizontally)")));
        lines.push(entry(String::from("Drag"), String::from("Select text in any panel, the result scrolls while dragging")));
        lines.push(entry(String::from("Right click"), String::from("Copy the selection")));
        lines.push(entry(String::from("Hover a link"), String::from(format!("Underlines it, {} copies it", key_bindings.generic.copy))));
        lines.push(entry(String::from("Double click tab"), String::from("Maximize the result panel, again to restore")));
        lines.push(entry(String::from("Footer keys"), String::from("Click them to run them")));
        lines.push(entry(String::from("Shift + drag"), String::from("Terminal native selection, if your terminal supports it")));

        lines
    }
}
