#[cfg(feature = "clipboard")]
use image::EncodableLayout;
use rayon::prelude::*;

use crate::app::app::App;
#[cfg(feature = "clipboard")]
use crate::app::clipboard;
use crate::models::response::ResponseContent;
use crate::tui::tui_logic::request::messages::get_displayed_messages;
use crate::tui::ui::result_tabs::RequestResultTabs;

impl App<'_> {
    /// Copy the current result tab content (body, messages, cookies, headers or console) to the clipboard
    pub fn copy_response_body_content_to_clipboard(&mut self) {
        let local_selected_request = self.get_selected_request_as_local();
        let selected_request = local_selected_request.read();

        let text = match self.request_result_tab {
            RequestResultTabs::Body => match &selected_request.response.content {
                None => String::new(),
                Some(ResponseContent::Body(body)) => body.clone(),
                Some(ResponseContent::Image(image_response)) => {
                    #[cfg(feature = "clipboard")]
                    if let Some(image) = &image_response.image {
                        let rgba_image = image.to_rgba8();

                        let copied = clipboard::copy_image(arboard::ImageData {
                            width: rgba_image.width() as usize,
                            height: rgba_image.height() as usize,
                            bytes: rgba_image.as_bytes().to_vec().into()
                        });

                        drop(selected_request);

                        match copied {
                            true => self.toast("Image copied"),
                            false => self.toast("Could not copy image")
                        }
                    }

                    #[cfg(not(feature = "clipboard"))]
                    let _ = image_response;

                    return;
                }
            },
            RequestResultTabs::Messages => get_displayed_messages(&selected_request)
                .iter()
                .map(|m| format!(
                    "=== {} - New {} message from {} ===\n{}",
                    m.timestamp.format("%H:%M:%S %d/%m/%Y").to_string(),
                    m.details,
                    m.sender,
                    m.content
                ))
                .collect::<Vec<String>>()
                .join("\n"),
            RequestResultTabs::Cookies => selected_request.response.cookies.clone().unwrap_or_default(),
            RequestResultTabs::Headers => selected_request.response.headers
                .par_iter()
                .map(|(header, value)| format!("{}: {}\n", header, value))
                .collect(),
            RequestResultTabs::Console => match (&selected_request.console_output.pre_request_output, &selected_request.console_output.post_request_output) {
                (None, None) => String::new(),
                (Some(pre_request_console_output), None) => pre_request_console_output.clone(),
                (None, Some(post_request_console_output)) => post_request_console_output.clone(),
                (Some(pre_request_console_output), Some(post_request_console_output)) => format!("{}\n{}", pre_request_console_output, post_request_console_output)
            }
        };

        drop(selected_request);

        self.copy_to_clipboard(&text, "Copied");
    }

    pub fn get_max_line_length(&self, text: &str) -> usize {
        let mut max_length = text.par_lines().fold_with(0, |acc, line| acc + line.chars().count()).sum();

        if self.last_messages_area_size.0 > 0 {
            let max_width = (0.75 * self.last_messages_area_size.0 as f32) as usize + 1;
            if max_length > max_width {
                max_length = max_width;
            }
        }

        max_length
    }
}