use crate::app::files::theme::THEME;
use crate::tui::utils::centered_rect::centered_rect;
use crate::tui::utils::stateful::stateful_scrollbar::StatefulScrollbar;
use ratatui::layout::Direction::Vertical;
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::{Line, Style};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation};
use ratatui::Frame;

#[derive(Default)]
pub struct DisplayPopup {
    pub title: String,
    pub lines: Vec<Line<'static>>,
    pub content: String,
    pub vertical_scrollbar: StatefulScrollbar,
    pub horizontal_scrollbar: StatefulScrollbar,
}

impl DisplayPopup {
    /// Returns the popup area
    pub fn render(&mut self, frame: &mut Frame) -> ratatui::layout::Rect {
        let popup_block = Block::default()
            .title(format!("{} export", self.title))
            .borders(Borders::ALL)
            .fg(THEME.read().ui.main_foreground_color)
            .bg(THEME.read().ui.secondary_background_color);

        let area = centered_rect(100, 25, frame.area());

        frame.render_widget(Clear, area);
        frame.render_widget(popup_block, area);

        let layout = Layout::new(
            Vertical,
            vec![
                Constraint::Fill(1)
            ]
        )
            .vertical_margin(1)
            .horizontal_margin(2)
            .split(area);

        let content = Paragraph::new(self.lines.clone())
            .scroll((
                self.vertical_scrollbar.offset(),
                self.horizontal_scrollbar.offset()
            ))
            .fg(THEME.read().ui.font_color);

        frame.render_widget(content, layout[0]);

        let vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::new().fg(THEME.read().ui.font_color));
        let horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .style(Style::new().fg(THEME.read().ui.font_color))
            .thumb_symbol("■");

        frame.render_stateful_widget(
            vertical_scrollbar,
            area,
            &mut self.vertical_scrollbar.state
        );

        frame.render_stateful_widget(
            horizontal_scrollbar,
            area,
            &mut self.horizontal_scrollbar.state
        );

        area
    }
}