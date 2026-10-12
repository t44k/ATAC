use ratatui::widgets::{ScrollbarState};

#[derive(Default)]
pub struct StatefulScrollbar {
    pub scroll: usize,
    pub max_scroll: usize,
    pub state: ScrollbarState,
}

impl StatefulScrollbar {
    pub fn page_up(&mut self) {
        self.scroll_by(-1);
    }

    pub fn page_down(&mut self) {
        self.scroll_by(1);
    }

    pub fn scroll_by(&mut self, delta: isize) {
        self.set_scroll(self.scroll.saturating_add_signed(delta));
    }

    pub fn set_scroll(&mut self, scroll: usize) {
        self.scroll = scroll.min(self.max_scroll);
        self.state = self.state.position(self.scroll);
    }

    pub fn set_max_scroll(&mut self, max_scroll: usize) {
        self.max_scroll = max_scroll;
        self.state = self.state.content_length(max_scroll);

        if self.scroll > max_scroll {
            self.set_scroll(max_scroll);
        }
    }

    pub fn top(&mut self) {
        self.set_scroll(0);
    }

    pub fn bottom(&mut self) {
        self.set_scroll(self.max_scroll);
    }

    /// Scroll position as expected by ratatui widgets
    pub fn offset(&self) -> u16 {
        self.scroll.min(u16::MAX as usize) as u16
    }
}
