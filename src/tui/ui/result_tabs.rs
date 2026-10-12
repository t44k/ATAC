use chrono::Local;
use ratatui::layout::Direction::Vertical;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Alignment, Style};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::style::Modifier;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Tabs};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use strum::{Display, EnumIter, FromRepr};
use textwrap::wrap;
use throbber_widgets_tui::{Throbber, WhichUse, BRAILLE_DOUBLE};

use crate::app::app::App;
use crate::app::files::theme::THEME;
use crate::models::protocol::protocol::Protocol;
use crate::models::protocol::ws::ws::Sender;
use crate::models::request::Request;
use crate::tui::tui_logic::request::messages::get_displayed_messages;
use crate::models::response::ResponseContent;
use crate::tui::utils::centered_rect::centered_rect;
use crate::app::app::ResultViewKey;
use crate::tui::mouse::MouseTarget;
use crate::tui::mouse::selection::SelectionKind;
use crate::tui::mouse::tabs::tab_rects;
use crate::tui::utils::result_view::ResultView;

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Display, FromRepr, EnumIter)]
pub enum RequestResultTabs {
    #[default]
    #[strum(to_string = "Result body")]
    Body,
    #[strum(to_string = "Messages")]
    Messages,
    #[strum(to_string = "Cookies")]
    Cookies,
    #[strum(to_string = "Headers")]
    Headers,
    #[strum(to_string = "Console")]
    Console,
}

impl App<'_> {
    pub(super) fn render_request_result(&mut self, frame: &mut Frame, rect: Rect, request: &Request) {
        let request_result_layout = Layout::new(
            Vertical,
            [
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Fill(1)
            ]
        )
            .split(rect);


        // REQUEST RESULT TABS

        let allowed_tabs = match &request.protocol {
            Protocol::HttpRequest(_) => vec![
                RequestResultTabs::Body,
                RequestResultTabs::Cookies,
                RequestResultTabs::Headers,
                RequestResultTabs::Console
            ],
            Protocol::WsRequest(_) => vec![
                RequestResultTabs::Messages,
                RequestResultTabs::Cookies,
                RequestResultTabs::Headers,
                RequestResultTabs::Console
            ],
            Protocol::MqttRequest(_) => vec![
                RequestResultTabs::Messages,
                RequestResultTabs::Headers,
                RequestResultTabs::Console
            ]
        };

        let result_tabs: Vec<(RequestResultTabs, String)> = allowed_tabs
            .iter()
            .filter_map(|tab| {
                let text = match tab {
                    RequestResultTabs::Body => {
                        if let Some(duration) = &request.response.duration {
                            Some(format!("{} ({})", tab.to_string(), duration))
                        }
                        else {
                            Some(format!("{}", tab.to_string()))
                        }
                    },
                    RequestResultTabs::Messages => {
                        let messages_count = get_displayed_messages(request).len();

                        if messages_count > 0 {
                            Some(format!("{} ({})", tab.to_string(), messages_count))
                        }
                        else {
                            Some(format!("{}", tab.to_string()))
                        }
                    },
                    RequestResultTabs::Cookies | RequestResultTabs::Headers => Some(tab.to_string()),
                    RequestResultTabs::Console => {
                        match (&request.console_output.pre_request_output, &request.console_output.post_request_output) {
                            (None, None) => None,
                            (_, _) => Some(tab.to_string())
                        }
                    }
                };

                text.map(|text| (*tab, text))
            })
            .collect();

        let selected_result_tab_index = result_tabs
            .iter()
            .position(|(tab, _)| *tab == self.request_result_tab)
            .unwrap_or(0);

        let tab_titles: Vec<&String> = result_tabs.iter().map(|(_, title)| title).collect();

        for (rect, (tab, _)) in tab_rects(request_result_layout[0], &tab_titles).into_iter().zip(&result_tabs) {
            self.hit(rect, MouseTarget::ResultTab(*tab));
        }

        let result_tabs_widget = Tabs::new(tab_titles.iter().map(|title| Span::raw(title.as_str()).fg(THEME.read().ui.font_color)))
            .highlight_style(THEME.read().others.selection_highlight_color)
            .select(selected_result_tab_index)
            .block(
                Block::new().borders(Borders::BOTTOM)
                    .fg(THEME.read().ui.main_foreground_color)
            );

        frame.render_widget(result_tabs_widget, request_result_layout[0]);

        let content_area = request_result_layout[2];

        // If the selected request is currently pending
        if request.is_pending {
            let area = centered_rect(9, 1, content_area);

            self.result_throbber_state.calc_next();
            
            let throbber = Throbber::default()
                .label("Pending")
                .style(Style::new().fg(THEME.read().ui.secondary_foreground_color))
                .throbber_set(BRAILLE_DOUBLE)
                .use_type(WhichUse::Spin);

            frame.render_stateful_widget(throbber, area, &mut self.result_throbber_state);
            return;
        }

        // REQUEST RESULT STATUS CODE

        let status_code = match &request.response.status_code {
            None => "",
            Some(status_code) => status_code
        };

        let status_code_paragraph = Paragraph::new(status_code)
            .centered()
            .fg(THEME.read().ui.secondary_foreground_color);
        frame.render_widget(status_code_paragraph, request_result_layout[1]);
        self.hit(request_result_layout[1], MouseTarget::Text);


        // REQUEST RESULT CONTENT

        match self.request_result_tab {
            RequestResultTabs::Body => match &request.response.content {
                None => {},
                Some(ResponseContent::Body(body)) => {
                    // Highlighted lines are used instead when available
                    self.render_result_text(frame, content_area, || body.lines().map(|line| Line::raw(line.to_string())).collect());
                }
                Some(ResponseContent::Image(image_response)) => match &image_response.image {
                    _ if self.config.is_image_preview_disabled() => {
                        let image_disabled_paragraph = Paragraph::new("\nImage preview disabled").centered();
                        frame.render_widget(image_disabled_paragraph, content_area);
                    },
                    Some(image) => {
                        // Query the terminal graphics capabilities only once
                        let is_graphical_protocol_disabled = self.config.is_graphical_protocol_disabled();
                        let picker = self.image_picker.get_or_insert_with(|| match is_graphical_protocol_disabled {
                            true => Picker::halfblocks(),
                            false => Picker::from_query_stdio().unwrap_or(Picker::halfblocks())
                        });

                        let mut image_static = picker.new_resize_protocol(image.clone());

                        frame.render_stateful_widget(StatefulImage::default(), content_area, &mut image_static);
                    }
                    None => {
                        let image_error_paragraph = Paragraph::new("\nCould not decode image")
                            .centered()
                            .fg(THEME.read().ui.font_color);
                        frame.render_widget(image_error_paragraph, content_area);
                    }
                },
            },
            RequestResultTabs::Messages => self.render_result_messages(frame, content_area, request),
            RequestResultTabs::Cookies => {
                let cookies = request.response.cookies.clone().unwrap_or_default();

                self.render_result_text(frame, content_area, || cookies
                    .lines()
                    .map(|line| Line::raw(line.to_string()).fg(THEME.read().ui.font_color))
                    .collect()
                );
            },
            RequestResultTabs::Headers => {
                self.render_result_text(frame, content_area, || request.response.headers
                    .iter()
                    .map(|(header, value)|
                        Line::from(vec![
                            Span::raw(header.clone()).bold().fg(THEME.read().ui.secondary_foreground_color),
                            Span::raw(": ").fg(THEME.read().ui.secondary_foreground_color),
                            Span::raw(value.clone()).fg(THEME.read().ui.font_color)
                        ])
                    )
                    .collect()
                );
            },
            RequestResultTabs::Console => {
                let console_lines = self.syntax_highlighting.highlighted_console_output.clone();
                self.render_result_text(frame, content_area, || console_lines);
            }
        };
    }

    /// Render scrollable, selectable text with clickable links in the result panel.
    /// `logical_lines` is only called when the content needs to be rebuilt.
    fn render_result_text(&mut self, frame: &mut Frame, area: Rect, logical_lines: impl FnOnce() -> Vec<Line<'static>>) {
        let wrap = self.config.should_wrap_body();

        // Leave room for the scrollbars
        let text_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width.saturating_sub(1),
            height: area.height.saturating_sub(if wrap { 0 } else { 1 }),
        };

        if text_area.is_empty() {
            return;
        }

        let key = ResultViewKey {
            generation: self.result_generation,
            request: self.collections_tree.selected,
            tab: self.request_result_tab,
            wrap_width: wrap.then_some(text_area.width as usize),
        };

        if self.result_view_key != Some(key) {
            let previous_key = self.result_view_key;

            let lines = match self.request_result_tab {
                RequestResultTabs::Body if !self.config.is_syntax_highlighting_disabled() && self.syntax_highlighting.highlighted_body.is_some() => {
                    self.syntax_highlighting.highlighted_body.clone().unwrap()
                },
                _ => logical_lines()
            };

            self.result_view = ResultView::new(lines, key.wrap_width);
            self.result_view_key = Some(key);

            // Another request or tab is displayed
            if previous_key.is_none_or(|previous_key| previous_key.request != key.request || previous_key.tab != key.tab) {
                self.result_vertical_scrollbar.top();
                self.result_horizontal_scrollbar.top();
                self.selection = None;
            }
        }

        self.last_result_area = text_area;

        let line_count = self.result_view.lines.len();
        let height = text_area.height as usize;

        self.result_vertical_scrollbar.set_max_scroll(line_count.saturating_sub(height));
        self.result_vertical_scrollbar.state = self.result_vertical_scrollbar.state.viewport_content_length(height);

        match wrap {
            true => self.result_horizontal_scrollbar.set_max_scroll(0),
            false => self.result_horizontal_scrollbar.set_max_scroll(self.result_view.max_width.saturating_sub(text_area.width as usize)),
        }
        self.result_horizontal_scrollbar.state = self.result_horizontal_scrollbar.state.viewport_content_length(text_area.width as usize);

        let first_row = self.result_vertical_scrollbar.scroll;
        let first_column = self.result_horizontal_scrollbar.scroll;
        let last_row = (first_row + height).min(line_count);

        let visible_lines = self.result_view.lines[first_row..last_row].to_vec();

        let paragraph = Paragraph::new(visible_lines).scroll((0, self.result_horizontal_scrollbar.offset()));
        frame.render_widget(paragraph, text_area);

        self.hit(text_area, MouseTarget::ResultContent);

        let buffer = frame.buffer_mut();

        // LINK UNDER THE MOUSE

        if let Some(position) = self.mouse.position && text_area.contains(position) {
            let row = first_row + (position.y - text_area.y) as usize;
            let column = first_column + (position.x - text_area.x) as usize;

            if let Some(link) = self.result_view.link_at(row, column) {
                self.hovered_link = Some(link.url.clone());

                let link_color = THEME.read().others.link_color;

                for (row, start, end) in &link.segments {
                    if *row < first_row || *row >= last_row {
                        continue;
                    }

                    let y = text_area.y + (row - first_row) as u16;

                    for column in *start..*end {
                        if column < first_column || column >= first_column + text_area.width as usize {
                            continue;
                        }

                        let cell = &mut buffer[(text_area.x + (column - first_column) as u16, y)];
                        cell.set_fg(link_color);
                        cell.modifier.insert(Modifier::UNDERLINED);
                    }
                }
            }
        }

        // SELECTION

        if let Some(selection) = &self.selection && selection.kind == SelectionKind::ResultContent {
            for row in first_row..last_row {
                let Some((start, end)) = selection.columns_in_row(row, self.result_view.row_width(row).max(1)) else {
                    continue;
                };

                let y = text_area.y + (row - first_row) as u16;

                for column in start.max(first_column)..=end.min(first_column + text_area.width as usize - 1) {
                    let cell = &mut buffer[(text_area.x + (column - first_column) as u16, y)];
                    cell.modifier.insert(Modifier::REVERSED);
                }
            }
        }

        // SCROLLBARS

        let vertical_scrollbar_area = Rect::new(area.right() - 1, area.y, 1, text_area.height);
        let horizontal_scrollbar_area = Rect::new(area.x, area.bottom() - 1, text_area.width, 1);

        if self.result_vertical_scrollbar.max_scroll > 0 {
            let result_vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::new().fg(THEME.read().ui.font_color));

            frame.render_stateful_widget(result_vertical_scrollbar, vertical_scrollbar_area, &mut self.result_vertical_scrollbar.state);
            self.hit(vertical_scrollbar_area, MouseTarget::ResultVerticalScrollbar);
        }

        if !wrap && self.result_horizontal_scrollbar.max_scroll > 0 {
            let result_horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .style(Style::new().fg(THEME.read().ui.font_color))
                .thumb_symbol("■"); // Better than the default full block

            frame.render_stateful_widget(result_horizontal_scrollbar, horizontal_scrollbar_area, &mut self.result_horizontal_scrollbar.state);
            self.hit(horizontal_scrollbar_area, MouseTarget::ResultHorizontalScrollbar);
        }
    }

    fn render_result_messages(&mut self, frame: &mut Frame, area: Rect, request: &Request) {
        let displayed_messages = get_displayed_messages(request);

        let mut messages = vec![];
        let mut last_sender: Option<&Sender> = None;

        let inner_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        };

        self.last_messages_area_size = (inner_area.width, inner_area.height);
        self.last_result_area = inner_area;

        for message in &displayed_messages {
            let mut alignment = Alignment::Right;

            let content = &message.content;
            let max_length = self.get_max_line_length(content);
            let lines = wrap(&content, max_length);

            match message.sender {
                Sender::You => {
                    let bg = match request.protocol {
                        Protocol::WsRequest(_) => THEME.read().websocket.messages.you_background_color,
                        Protocol::MqttRequest(_) => THEME.read().mqtt.messages.you_background_color,
                        _ => unreachable!()
                    };

                    for line in lines {
                        let line = match line.is_empty() {
                            true => " ".repeat(max_length),
                            false => format!("{line:max_length$}")
                        };

                        messages.push(
                            Line::raw(line)
                                .fg(THEME.read().ui.font_color)
                                .bg(bg)
                                .alignment(alignment)
                        );
                    }
                },
                Sender::Server => {
                    alignment = Alignment::Left;

                    let (fg, bg) = match request.protocol {
                        Protocol::WsRequest(_) => (
                            THEME.read().websocket.messages.server_foreground_color,
                            THEME.read().websocket.messages.server_background_color,
                        ),
                        Protocol::MqttRequest(_) => (
                            THEME.read().mqtt.messages.server_foreground_color,
                            THEME.read().mqtt.messages.server_background_color,
                        ),
                        _ => unreachable!()
                    };

                    if last_sender != Some(&message.sender) {
                        messages.push(
                            Line::raw(message.sender.to_string())
                                .bold()
                                .fg(fg)
                                .alignment(alignment)
                        );
                    }

                    for line in lines {
                        let line = match line.is_empty() {
                            true => " ".repeat(max_length),
                            false => format!("{line:max_length$}")
                        };


                        messages.push(
                            Line::raw(line)
                                .fg(THEME.read().ui.font_color)
                                .bg(bg)
                                .alignment(alignment)
                        );
                    }
                }
            }

            let timestamp_format = match Local::now().date_naive() == message.timestamp.date_naive() {
                true => "%H:%M:%S",
                false => "%H:%M:%S %d/%m/%Y"
            };

            let timestamp = message.timestamp.format(timestamp_format).to_string();

            let fg_message = match request.protocol {
                Protocol::WsRequest(_) => THEME.read().websocket.messages.details_color,
                Protocol::MqttRequest(_) => THEME.read().mqtt.messages.details_color,
                _ => unreachable!()
            };

            messages.push(
                Line::raw(format!("{} {}", message.details, timestamp))
                    .fg(fg_message)
                    .alignment(alignment)
            );

            last_sender = Some(&message.sender);
        }

        self.result_vertical_scrollbar.set_max_scroll(messages.len().saturating_sub(inner_area.height as usize));
        self.result_horizontal_scrollbar.set_max_scroll(0);

        let messages_paragraph = Paragraph::new(messages)
            .scroll((self.result_vertical_scrollbar.offset(), 0));

        frame.render_widget(messages_paragraph, inner_area);
        self.hit(inner_area, MouseTarget::Text);

        if self.result_vertical_scrollbar.max_scroll > 0 {
            let vertical_scrollbar_area = Rect::new(area.right().saturating_sub(1), area.y, 1, area.height);

            let result_vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::new().fg(THEME.read().ui.font_color));

            frame.render_stateful_widget(result_vertical_scrollbar, vertical_scrollbar_area, &mut self.result_vertical_scrollbar.state);
            self.hit(vertical_scrollbar_area, MouseTarget::ResultVerticalScrollbar);
        }
    }
}
