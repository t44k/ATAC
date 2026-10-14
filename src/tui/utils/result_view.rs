//! Pre-computed visual lines of the result panel.
//!
//! The lines are wrapped here instead of by ratatui so that screen positions can be mapped back to the
//! text: needed to scroll exactly, select and copy text, and find links under the mouse.

use lazy_static::lazy_static;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use regex::Regex;
use unicode_width::UnicodeWidthChar;

const TAB_WIDTH: usize = 4;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r#"https?://[^\s"'<>`{}|\\^\[\]]+"#).unwrap();
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub url: String,
    /// (visual row, first column, last column exclusive)
    pub segments: Vec<(usize, usize, usize)>,
}

impl Link {
    pub fn contains(&self, row: usize, column: usize) -> bool {
        self.segments.iter().any(|(r, start, end)| *r == row && column >= *start && column < *end)
    }
}

#[derive(Default)]
pub struct ResultView {
    /// Styled lines as displayed
    pub lines: Vec<Line<'static>>,
    /// Characters of each displayed line with their column and width (tabs are kept to be copied as is)
    cells: Vec<Vec<(usize, usize, char)>>,
    /// The displayed line continues the previous one (soft wrap)
    pub continuation: Vec<bool>,
    pub links: Vec<Link>,
    /// Width of the widest displayed line
    pub max_width: usize,
}

impl ResultView {
    /// Build the visual lines, wrapping at `wrap_width` columns when set
    pub fn new(logical_lines: Vec<Line<'static>>, wrap_width: Option<usize>) -> ResultView {
        let mut view = ResultView::default();

        for logical_line in logical_lines {
            view.push_logical_line(logical_line, wrap_width);
        }

        view
    }

    fn push_logical_line(&mut self, logical_line: Line<'static>, wrap_width: Option<usize>) {
        let line_style = logical_line.style;
        let alignment = logical_line.alignment;

        let mut text = String::new();
        // Visual row, column and width of each char of `text`, indexed by char
        let mut char_positions: Vec<(usize, usize, usize)> = vec![];

        let mut row_spans: Vec<Span<'static>> = vec![];
        let mut row_cells: Vec<(usize, usize, char)> = vec![];
        let mut current_style: Option<Style> = None;
        let mut current_content = String::new();
        let mut column = 0;
        let mut is_continuation = false;

        for span in logical_line.spans {
            let style = line_style.patch(span.style);

            for char in span.content.chars() {
                let width = match char {
                    '\t' => TAB_WIDTH,
                    char => char.width().unwrap_or(0)
                };

                if let Some(wrap_width) = wrap_width && wrap_width > 0 && column > 0 && column + width > wrap_width {
                    if !current_content.is_empty() {
                        row_spans.push(Span::styled(std::mem::take(&mut current_content), current_style.unwrap_or_default()));
                    }

                    self.push_row(std::mem::take(&mut row_spans), std::mem::take(&mut row_cells), column, is_continuation, alignment);
                    column = 0;
                    is_continuation = true;
                }

                if current_style != Some(style) {
                    if !current_content.is_empty() {
                        row_spans.push(Span::styled(std::mem::take(&mut current_content), current_style.unwrap_or_default()));
                    }
                    current_style = Some(style);
                }

                match char {
                    '\t' => current_content.push_str(&" ".repeat(TAB_WIDTH)),
                    char => current_content.push(char)
                }

                text.push(char);
                char_positions.push((self.lines.len(), column, width));
                row_cells.push((column, width, char));
                column += width;
            }
        }

        if !current_content.is_empty() {
            row_spans.push(Span::styled(current_content, current_style.unwrap_or_default()));
        }

        self.push_row(row_spans, row_cells, column, is_continuation, alignment);
        self.find_links(&text, &char_positions);
    }

    fn push_row(&mut self, spans: Vec<Span<'static>>, cells: Vec<(usize, usize, char)>, width: usize, is_continuation: bool, alignment: Option<ratatui::layout::HorizontalAlignment>) {
        let mut line = Line::from(spans);
        line.alignment = alignment;

        self.lines.push(line);
        self.cells.push(cells);
        self.continuation.push(is_continuation);
        self.max_width = self.max_width.max(width);
    }

    fn find_links(&mut self, text: &str, char_positions: &[(usize, usize, usize)]) {
        if !text.contains("http") {
            return;
        }

        // Chars are counted from the previous match only, a long line can contain many links
        let mut counted_bytes = 0;
        let mut counted_chars = 0;

        for found in URL_REGEX.find_iter(text) {
            let url = found.as_str().trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '\'', '"']);

            if url.len() <= "https://".len() {
                continue;
            }

            counted_chars += text[counted_bytes..found.start()].chars().count();
            counted_bytes = found.start();

            let first_char = counted_chars;
            let char_count = url.chars().count();

            let mut segments: Vec<(usize, usize, usize)> = vec![];

            for &(row, column, width) in &char_positions[first_char..first_char + char_count] {
                match segments.last_mut() {
                    Some((last_row, _, end)) if *last_row == row => *end = column + width,
                    _ => segments.push((row, column, column + width))
                }
            }

            self.links.push(Link {
                url: url.to_string(),
                segments,
            });
        }
    }

    pub fn link_at(&self, row: usize, column: usize) -> Option<&Link> {
        self.links.iter().find(|link| link.contains(row, column))
    }

    /// Text between two (row, column) positions, inclusive. Soft wrapped lines are joined back together.
    pub fn text_between(&self, start: (usize, usize), end: (usize, usize)) -> String {
        let mut text = String::new();

        if self.lines.is_empty() {
            return text;
        }

        let last_row = end.0.min(self.lines.len() - 1);

        for row in start.0..=last_row {
            if row > start.0 && !self.continuation[row] {
                text.push('\n');
            }

            let first_column = if row == start.0 { start.1 } else { 0 };
            let last_column = if row == end.0 { end.1 } else { usize::MAX };

            for (column, width, char) in &self.cells[row] {
                // Keep the char if any of its cells is selected
                if column + width.max(&1) > first_column && *column <= last_column {
                    text.push(*char);
                }
            }
        }

        text
    }

    /// Number of columns of a row
    pub fn row_width(&self, row: usize) -> usize {
        match self.cells.get(row).and_then(|cells| cells.last()) {
            None => 0,
            Some((column, width, _)) => column + width
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Stylize;

    fn plain(view: &ResultView) -> Vec<String> {
        view.lines.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn keeps_lines_without_wrap() {
        let view = ResultView::new(vec![Line::raw("hello world"), Line::raw("")], None);

        assert_eq!(plain(&view), vec!["hello world", ""]);
        assert_eq!(view.max_width, 11);
        assert_eq!(view.continuation, vec![false, false]);
    }

    #[test]
    fn wraps_on_width_and_keeps_styles() {
        let line = Line::from(vec!["abc".red(), "defgh".blue()]);
        let view = ResultView::new(vec![line], Some(4));

        assert_eq!(plain(&view), vec!["abcd", "efgh"]);
        assert_eq!(view.continuation, vec![false, true]);
        assert_eq!(view.lines[0].spans.len(), 2);
        assert_eq!(view.lines[0].spans[1].content, "d");
    }

    #[test]
    fn copies_text_across_soft_wraps_without_new_lines() {
        let view = ResultView::new(vec![Line::raw("abcdefgh"), Line::raw("ijkl")], Some(4));

        assert_eq!(view.text_between((0, 2), (2, 1)), "cdefgh\nij");
    }

    #[test]
    fn detects_links_and_trims_punctuation() {
        let view = ResultView::new(vec![Line::raw(r#"{"next": "https://api.example.com/page/2"},"#), Line::raw("see https://example.com.")], None);

        assert_eq!(view.links.len(), 2);
        assert_eq!(view.links[0].url, "https://api.example.com/page/2");
        assert_eq!(view.links[0].segments, vec![(0, 10, 40)]);
        assert_eq!(view.links[1].url, "https://example.com");
        assert_eq!(view.link_at(1, 4).unwrap().url, "https://example.com");
        assert!(view.link_at(1, 3).is_none());
    }

    #[test]
    fn links_split_by_wrap_have_one_segment_per_row() {
        let view = ResultView::new(vec![Line::raw("go https://a.io/xyz")], Some(10));

        assert_eq!(view.links[0].url, "https://a.io/xyz");
        assert_eq!(view.links[0].segments, vec![(0, 3, 10), (1, 0, 9)]);
        assert_eq!(view.link_at(1, 2).unwrap().url, "https://a.io/xyz");
    }

    #[test]
    fn finds_several_links_on_a_line_after_wide_chars() {
        let view = ResultView::new(vec![Line::raw("日本 https://a.io é https://b.io")], None);

        assert_eq!(view.links.len(), 2);
        assert_eq!(view.links[0].segments, vec![(0, 5, 17)]);
        assert_eq!(view.links[1].url, "https://b.io");
        assert_eq!(view.links[1].segments, vec![(0, 20, 32)]);
    }

    #[test]
    fn handles_wide_chars_and_tabs() {
        let view = ResultView::new(vec![Line::raw("日本\tx")], None);

        assert_eq!(view.row_width(0), 9);
        assert_eq!(view.text_between((0, 1), (0, 8)), "日本\tx");
    }
}

