//! Text selection made with the mouse.
//!
//! Most of the UI is selected directly on the rendered buffer, so any widget can be selected and copied.
//! The result panel is selected in content coordinates instead, so a selection can be larger than the viewport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionKind {
    /// (row, column) are screen coordinates, clamped to `region`
    Screen,
    /// (row, column) are coordinates in the result view lines
    ResultContent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub kind: SelectionKind,
    /// Screen area the selection started in
    pub region: Rect,
    /// (row, column) where the selection started
    pub anchor: (usize, usize),
    /// (row, column) where the selection currently ends, inclusive
    pub head: (usize, usize),
}

impl Selection {
    /// Start and end of the selection, in reading order
    pub fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        }
        else {
            (self.head, self.anchor)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Selected columns of a row, inclusive. `row_width` is used for rows fully covered by the selection.
    pub fn columns_in_row(&self, row: usize, row_width: usize) -> Option<(usize, usize)> {
        let ((start_row, start_col), (end_row, end_col)) = self.ordered();

        if row < start_row || row > end_row {
            return None;
        }

        let first = if row == start_row { start_col } else { 0 };
        let last = if row == end_row { end_col } else { row_width.saturating_sub(1) };

        match first <= last {
            true => Some((first, last)),
            false => None
        }
    }
}

/// Highlight the selected cells of a screen selection
pub fn highlight_screen_selection(buffer: &mut Buffer, selection: &Selection) {
    let region = selection.region.intersection(buffer.area);

    for y in region.top()..region.bottom() {
        let Some((first, last)) = selection.columns_in_row(y as usize, region.right() as usize) else {
            continue;
        };

        let first = (first as u16).max(region.left());
        let last = (last as u16).min(region.right().saturating_sub(1));

        for x in first..=last {
            let cell = &mut buffer[(x, y)];
            cell.modifier.insert(Modifier::REVERSED);
        }
    }
}

/// Text of a screen selection, read from the rendered buffer.
/// Trailing spaces of each line are removed and wide characters are only counted once.
pub fn screen_selection_text(buffer: &Buffer, selection: &Selection) -> String {
    let region = selection.region.intersection(buffer.area);
    let mut lines = vec![];

    for y in region.top()..region.bottom() {
        let Some((first, last)) = selection.columns_in_row(y as usize, region.right() as usize) else {
            continue;
        };

        let first = (first as u16).max(region.left());
        let last = (last as u16).min(region.right().saturating_sub(1));

        let mut line = String::new();
        let mut x = first;

        while x <= last {
            let symbol = buffer[(x, y)].symbol();
            line.push_str(symbol);

            // Wide characters span multiple cells, skip the cells they cover
            x += (symbol.width() as u16).max(1);
        }

        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;
    use ratatui::widgets::Widget;

    fn buffer_with(lines: &[&str]) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, lines.len() as u16));

        for (i, line) in lines.iter().enumerate() {
            Line::raw(*line).render(Rect::new(0, i as u16, 12, 1), &mut buffer);
        }

        buffer
    }

    #[test]
    fn extracts_multi_line_screen_selection() {
        let buffer = buffer_with(&["hello world", "second line", "third"]);

        let selection = Selection {
            kind: SelectionKind::Screen,
            region: buffer.area,
            anchor: (0, 6),
            head: (1, 5),
        };

        assert_eq!(screen_selection_text(&buffer, &selection), "world\nsecond");
    }

    #[test]
    fn reversed_selection_is_ordered() {
        let buffer = buffer_with(&["hello world", "second line"]);

        let selection = Selection {
            kind: SelectionKind::Screen,
            region: buffer.area,
            anchor: (1, 5),
            head: (0, 6),
        };

        assert_eq!(screen_selection_text(&buffer, &selection), "world\nsecond");
    }

    #[test]
    fn selection_is_clamped_to_its_region() {
        let buffer = buffer_with(&["left | right", "left | right"]);

        let selection = Selection {
            kind: SelectionKind::Screen,
            region: Rect::new(7, 0, 5, 2),
            anchor: (0, 0),
            head: (1, 20),
        };

        assert_eq!(screen_selection_text(&buffer, &selection), "right\nright");
    }

    #[test]
    fn wide_characters_are_copied_once() {
        let buffer = buffer_with(&["日本語 ok"]);

        let selection = Selection {
            kind: SelectionKind::Screen,
            region: buffer.area,
            anchor: (0, 0),
            head: (0, 11),
        };

        assert_eq!(screen_selection_text(&buffer, &selection), "日本語 ok");
    }
}
