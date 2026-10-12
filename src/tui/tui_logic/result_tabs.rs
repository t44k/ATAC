use std::str::Lines;
use strum::IntoEnumIterator;
use ratatui::prelude::{Line, Stylize};
use crate::app::app::App;
use crate::app::files::theme::THEME;
use crate::models::protocol::http::body::find_file_format_in_content_type;
use crate::models::protocol::protocol::Protocol;
use crate::tui::ui::result_tabs::RequestResultTabs;
use crate::models::response::ResponseContent;
use crate::tui::utils::syntax_highlighting::highlight;

impl App<'_> {
    pub fn tui_next_request_result_tab(&mut self) {
        let local_selected_request = self.get_selected_request_as_local();
        let selected_request = local_selected_request.read();

        self.request_result_tab = match self.request_result_tab {
            RequestResultTabs::Body => RequestResultTabs::Cookies,
            // MQTT has no cookies
            RequestResultTabs::Messages => match selected_request.protocol {
                Protocol::MqttRequest(_) => RequestResultTabs::Headers,
                _ => RequestResultTabs::Cookies
            },
            RequestResultTabs::Cookies => RequestResultTabs::Headers,
            RequestResultTabs::Headers => {
                let local_selected_request = self.get_selected_request_as_local();
                let selected_request = local_selected_request.read();

                match (&selected_request.console_output.pre_request_output, &selected_request.console_output.post_request_output) {
                    (None, None) => match selected_request.protocol {
                        Protocol::HttpRequest(_) => RequestResultTabs::Body,
                        Protocol::WsRequest(_) => RequestResultTabs::Messages,
                        Protocol::MqttRequest(_) => RequestResultTabs::Messages
                    },
                    (_, _) => RequestResultTabs::Console
                }
            },
            RequestResultTabs::Console => match selected_request.protocol {
                Protocol::HttpRequest(_) => RequestResultTabs::Body,
                Protocol::WsRequest(_) => RequestResultTabs::Messages,
                Protocol::MqttRequest(_) => RequestResultTabs::Messages
            }
        };

        *self.received_response.lock() = true;
    }

    pub fn tui_previous_request_result_tab(&mut self) {
        let start = self.request_result_tab;
        let mut previous = start;

        // Go around the tabs once, the previous tab is the one before the start
        for _ in 0..RequestResultTabs::iter().count() {
            self.tui_next_request_result_tab();

            if self.request_result_tab == start {
                break;
            }

            previous = self.request_result_tab;
        }

        self.request_result_tab = previous;
    }

    pub fn tui_update_request_result_tab(&mut self) {
        let local_selected_request = self.get_selected_request_as_local();
        let selected_request = local_selected_request.read();

        if self.request_result_tab == RequestResultTabs::Console && selected_request.console_output.pre_request_output.is_none() && selected_request.console_output.post_request_output.is_none() {
            self.request_result_tab = match selected_request.protocol {
                Protocol::HttpRequest(_) => RequestResultTabs::Body,
                Protocol::WsRequest(_) => RequestResultTabs::Messages,
                Protocol::MqttRequest(_) => RequestResultTabs::Messages
            };
        }
        else {
            match selected_request.protocol {
                Protocol::HttpRequest(_) if self.request_result_tab == RequestResultTabs::Messages => self.request_result_tab = RequestResultTabs::Body,
                Protocol::WsRequest(_) if self.request_result_tab == RequestResultTabs::Body => self.request_result_tab = RequestResultTabs::Messages,
                Protocol::MqttRequest(_) if matches!(self.request_result_tab, RequestResultTabs::Body | RequestResultTabs::Cookies) => self.request_result_tab = RequestResultTabs::Messages,
                _ => {}
            };
        }
    }

    pub fn tui_highlight_response_body_and_console(&mut self) {
        let local_selected_request = self.get_selected_request_as_local();
        let selected_request = local_selected_request.write();

        // The result view will be rebuilt on next render
        self.result_generation += 1;

        self.syntax_highlighting.highlighted_body = None;
        self.syntax_highlighting.highlighted_console_output = vec![];

        if let Some(file_format) = find_file_format_in_content_type(&selected_request.response.headers) {
            if let Some(ResponseContent::Body(response_content)) = &selected_request.response.content.as_ref() {
                self.syntax_highlighting.highlighted_body = highlight(response_content, &file_format);
            }
        }

        if let Some(pre_request_console_output) = &selected_request.console_output.pre_request_output {
            let mut highlighted_console_output = highlight(pre_request_console_output, "json").unwrap();

            highlighted_console_output.insert(0, Line::default());
            highlighted_console_output.insert(1, Line::raw("----- Pre-request script start -----").fg(THEME.read().ui.secondary_foreground_color).centered());
            highlighted_console_output.push(Line::raw("----- Pre-request script end -----").fg(THEME.read().ui.secondary_foreground_color).centered());

            self.syntax_highlighting.highlighted_console_output.extend(highlighted_console_output);
        }

        if let Some(post_request_console_output) = &selected_request.console_output.post_request_output {
            let mut highlighted_console_output = highlight(post_request_console_output, "json").unwrap();

            highlighted_console_output.insert(0, Line::default());
            highlighted_console_output.insert(1, Line::raw("----- Post-request script start -----").fg(THEME.read().ui.secondary_foreground_color).centered());
            highlighted_console_output.push(Line::raw("----- Post-request script end -----").fg(THEME.read().ui.secondary_foreground_color).centered());

            self.syntax_highlighting.highlighted_console_output.extend(highlighted_console_output);
        }
    }

    pub fn get_max_str_len(lines: Lines) -> usize {
        let mut max_tmp = 0;

        for line in lines {
            if line.len() > max_tmp {
                max_tmp = line.len();
            }
        }

        return max_tmp;
    }
}