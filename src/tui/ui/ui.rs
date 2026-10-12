use std::time::Duration;

use crokey::KeyCombination;
use ratatui::backend::Backend;
use ratatui::{Frame, Terminal};
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Position, Rect};
use ratatui::layout::Direction::{Horizontal, Vertical};
use ratatui::prelude::{Modifier};
use ratatui::style::{Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::app::{App};
use crate::app::files::theme::THEME;
use crate::models::protocol::protocol::Protocol;
use crate::tui::app_states::AppState::*;
use crate::app::files::key_bindings::KEY_BINDINGS;
use crate::tui::app_states::AVAILABLE_EVENTS;
use crate::tui::events::AppEvent;
use crate::tui::mouse::MouseTarget;
use crate::tui::mouse::selection::{highlight_screen_selection, screen_selection_text, SelectionKind};

const TOAST_DURATION: Duration = Duration::from_secs(3);

/// Small text displayed next to the mouse cursor
fn render_floating_hint(frame: &mut Frame, position: Position, text: &str) {
    let screen = frame.area();
    let width = (text.chars().count() as u16).min(screen.width);

    let x = position.x.min(screen.right().saturating_sub(width));
    let y = match position.y + 1 < screen.bottom() {
        true => position.y + 1,
        false => position.y.saturating_sub(1)
    };

    let area = Rect::new(x, y, width, 1);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text.to_string())
            .fg(THEME.read().ui.font_color)
            .bg(THEME.read().ui.secondary_background_color),
        area
    );
}


impl App<'_> {
    fn ui(&mut self, frame: &mut Frame) {
        self.hit_map.clear();
        self.hovered_link = None;
        self.last_result_area = Rect::default();

        if let Some(bg_color) = THEME.read().ui.app_background {
            let test = Block::new().bg(bg_color);

            frame.render_widget(test, frame.area());
        }


        // MAIN LAYOUT

        let main_layout = Layout::new(
            Vertical,
            [
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ],
        )
            .split(frame.area());


        // HEADER

        let header = Block::new()
            .title("* ATAC *")
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::ITALIC)
            .title_alignment(Alignment::Center)
            .borders(Borders::TOP);

        frame.render_widget(header, main_layout[0]);
        self.hit(main_layout[0], MouseTarget::Text);

        // INNER LAYOUT

        let inner_layout = Layout::new(
            Horizontal,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(80)
            ],
        )
            .split(main_layout[1]);

        // LEFT LAYOUT

        match self.environments.is_empty() {
            // No environments
            true => {
                let env_and_collections_layout = Layout::new(
                    Vertical,
                    [
                        Constraint::Fill(1)
                    ]
                )
                    .split(inner_layout[0]);

                // COLLECTION

                self.render_collections(frame, env_and_collections_layout[0]);
            }
            // At least one environment
            false => {
                let env_and_collections_layout = Layout::new(
                    Vertical,
                    [
                        Constraint::Length(3),
                        Constraint::Fill(1)
                    ]
                )
                    .split(inner_layout[0]);

                // ENVIRONMENTS

                self.render_environments(frame, env_and_collections_layout[0]);

                // COLLECTION

                self.render_collections(frame, env_and_collections_layout[1]);
            }
        }

        // REQUEST

        match self.collections_tree.selected {
            None => self.render_homepage(frame, inner_layout[1]),
            Some(selection) => {
                let selected_request = self.get_request_as_local_from_indexes(&selection).read().clone();

                match selected_request.protocol {
                    Protocol::HttpRequest(_) => self.render_http_request(frame, inner_layout[1], selected_request),
                    Protocol::WsRequest(_) | Protocol::MqttRequest(_) => self.render_ws_or_mqtt_request(frame, inner_layout[1], selected_request),
                }
            }
        }

        // FOOTER

        if let Some((_, time)) = &self.toast && time.elapsed() > TOAST_DURATION {
            self.toast = None;
        }

        let state_line = match &self.toast {
            Some((message, _)) => Line::from(message.clone()).fg(THEME.read().others.selection_highlight_color),
            None => self.get_state_line()
        };

        let footer_left = Block::new()
            .title(state_line)
            .title_alignment(Alignment::Left);

        frame.render_widget(footer_left, main_layout[2]);
        self.hit(main_layout[2], MouseTarget::Text);

        self.render_footer_keys(frame, main_layout[2]);

        // POPUPS

        let is_popup_displayed = !matches!(
            self.state,
            Normal | SelectedRequest |
            EditingRequestUrl | EditingRequestParam |
            EditingRequestAuthBasicUsername | EditingRequestAuthBasicPassword | EditingRequestAuthBearerToken |
            EditingRequestAuthJwtSecret | EditingRequestAuthJwtPayload |
            EditingRequestAuthDigestUsername | EditingRequestAuthDigestPassword | EditingRequestAuthDigestDomains |
            EditingRequestAuthDigestRealm | EditingRequestAuthDigestNonce | EditingRequestAuthDigestOpaque |
            EditingRequestHeader | EditingRequestBodyTable | EditingRequestBodyFile | EditingRequestBodyString |
            EditingRequestMessage | EditingMqttFormField | EditingMqttSubscription |
            EditingPreRequestScript | EditingPostRequestScript
        );

        // Clicking out of a popup closes it
        if is_popup_displayed {
            self.hit(frame.area(), MouseTarget::Backdrop);
        }

        match self.state {
            DisplayingEnvEditor | EditingEnvVariable => self.render_env_editor_popup(frame),
            DisplayingCookies | EditingCookies => self.render_cookies_popup(frame),
            DisplayingLogs => self.render_logs_popup(frame),
            ChoosingElementToCreate => self.render_creating_element_popup(frame),
            CreatingNewCollection => self.render_creating_new_collection_popup(frame),
            CreatingNewRequest => self.render_creating_new_request_popup(frame),
            DeletingCollection => self.render_deleting_collection_popup(frame),
            DeletingRequest => self.render_deleting_request_popup(frame),
            EditingRequestSettings => self.render_request_settings_popup(frame),
            RenamingCollection => self.render_renaming_collection_popup(frame),
            RenamingRequest => self.render_renaming_request_popup(frame),
            ChoosingRequestExportFormat => self.render_export_format_popup(frame),
            DisplayingRequestExport => {
                let area = self.display_request_export.render(frame);
                self.hit(area, MouseTarget::Popup);
                self.hit(area.inner(Margin::new(2, 1)), MouseTarget::ExportContent);
            },
            _ => {}
        }

        if self.should_display_help {
            self.render_help_popup(frame);   
        }

        // SELECTION

        self.selection_text = None;

        if let Some(selection) = &self.selection && selection.kind == SelectionKind::Screen {
            self.selection_text = Some(screen_selection_text(frame.buffer_mut(), selection));
            highlight_screen_selection(frame.buffer_mut(), selection);
        }

        // LINK HINT

        if self.hovered_link.is_some() && let Some(position) = self.mouse.position {
            render_floating_hint(frame, position, &format!(" {} copy link ", KEY_BINDINGS.read().generic.copy));
        }
    }

    /// Available keys on the right of the footer, each one can be clicked
    fn render_footer_keys(&mut self, frame: &mut Frame, area: Rect) {
        let events = AVAILABLE_EVENTS.read().clone();
        let fg = THEME.read().ui.secondary_foreground_color;
        let bg = THEME.read().ui.secondary_background_color;

        let mut keys: Vec<(KeyCombination, Vec<Span>)> = events
            .iter()
            .filter_map(|event| {
                let is_documentation = matches!(event, AppEvent::Documentation(_));
                let event_key_bindings = event.get_event_key_bindings();

                event_key_bindings
                    .to_spans(fg, bg, true, is_documentation)
                    .map(|spans| (event_key_bindings.keys[0], spans))
            })
            .collect();

        // No trailing space
        if let Some((_, spans)) = keys.last_mut() {
            spans.pop();
        }

        let width: u16 = keys.iter().flat_map(|(_, spans)| spans).map(|span| span.width() as u16).sum();
        let mut x = area.right().saturating_sub(width).max(area.x);

        for (key, spans) in keys {
            let key_width: u16 = spans.iter().map(|span| span.width() as u16).sum();
            let key_area = Rect::new(x, area.y, key_width, 1).intersection(area);

            frame.render_widget(Line::from(spans), key_area);
            self.hit(key_area, MouseTarget::FooterKey(key));

            x = x.saturating_add(key_width);
        }
    }

    pub fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        terminal.draw(|frame | self.ui(frame))?;
        Ok(())
    }
}