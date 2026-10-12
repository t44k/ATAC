use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

/// Clickable area of each title of a ratatui `Tabs` widget using the default padding (1 space each side) and divider (1 column).
/// `ratatui::widgets::Tabs` does not expose its layout, so it is recomputed here.
pub fn tab_rects<S: AsRef<str>>(area: Rect, titles: &[S]) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(titles.len());
    let mut x = area.x;

    for title in titles {
        if x >= area.right() {
            break;
        }

        let width = 1 + title.as_ref().width() as u16 + 1;
        let width = width.min(area.right() - x);

        rects.push(Rect::new(x, area.y, width, 1));

        // Title, paddings and divider
        x = x.saturating_add(width + 1);
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_tab_positions_with_padding_and_divider() {
        let rects = tab_rects(Rect::new(10, 3, 80, 2), &["Params", "Auth", "Headers (2)"]);

        assert_eq!(rects, vec![
            Rect::new(10, 3, 8, 1),
            Rect::new(19, 3, 6, 1),
            Rect::new(26, 3, 13, 1),
        ]);
    }

    #[test]
    fn clips_tabs_to_the_area() {
        let rects = tab_rects(Rect::new(0, 0, 12, 1), &["Params", "Auth", "Headers"]);

        assert_eq!(rects, vec![Rect::new(0, 0, 8, 1), Rect::new(9, 0, 3, 1)]);
    }
}
