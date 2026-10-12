use std::io::Stdout;
use std::time::{Duration, Instant};

use crokey::KeyCombination;
use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use crate::app::app::App;
use crate::app::files::key_bindings::KEY_BINDINGS;
use crate::tui::app_states::AppState;
use crate::tui::app_states::AVAILABLE_EVENTS;
use crate::tui::mouse::selection::{Selection, SelectionKind};
use crate::tui::mouse::{DragOrigin, MouseTarget, TableId};
use crate::tui::ui::param_tabs::param_tabs::RequestParamsTabs;
use crate::tui::ui::views::RequestView;
use crate::tui::utils::stateful::stateful_custom_table::StatefulCustomTable;
use crate::tui::utils::stateful::text_input::TextInput;

const DOUBLE_CLICK_DELAY: Duration = Duration::from_millis(400);
const WHEEL_LINES: isize = 3;

impl App<'_> {
    pub async fn handle_mouse(&mut self, event: MouseEvent, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        let position = Position::new(event.column, event.row);
        self.mouse.position = Some(position);

        let horizontal = event.modifiers.contains(KeyModifiers::SHIFT);

        match event.kind {
            MouseEventKind::ScrollUp => self.scroll_at(position, -WHEEL_LINES, horizontal, event),
            MouseEventKind::ScrollDown => self.scroll_at(position, WHEEL_LINES, horizontal, event),
            MouseEventKind::ScrollLeft => self.scroll_at(position, -WHEEL_LINES, true, event),
            MouseEventKind::ScrollRight => self.scroll_at(position, WHEEL_LINES, true, event),
            MouseEventKind::Down(MouseButton::Left) => self.mouse_down(position, event, terminal).await,
            MouseEventKind::Drag(MouseButton::Left) => self.mouse_drag(position, event),
            MouseEventKind::Up(MouseButton::Left) => self.mouse_up(event),
            MouseEventKind::Down(MouseButton::Right) => {
                self.copy_selection_or_link();
            },
            _ => {}
        }
    }

    async fn mouse_down(&mut self, position: Position, event: MouseEvent, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        let now = Instant::now();
        let is_double_click = matches!(self.mouse.last_click, Some((time, last_position)) if now.duration_since(time) < DOUBLE_CLICK_DELAY && last_position == position);
        self.mouse.last_click = Some((now, position));

        self.selection = None;
        self.mouse.drag_origin = None;

        let target = self.target_at(position);

        if self.should_display_help {
            match target {
                Some((region, MouseTarget::Help)) => self.mouse.drag_origin = Some(DragOrigin { position, region, target: MouseTarget::Help, into_editor: false }),
                _ => self.should_display_help = false
            }
            return;
        }

        // Clicking inside the text input being edited moves its cursor and starts a selection in it
        if let Some(input) = self.active_text_input() && input.last_area.contains(position) {
            let region = input.last_area;
            input.mouse_event(event);
            self.mouse.drag_origin = Some(DragOrigin { position, region, target: MouseTarget::Text, into_editor: true });
            return;
        }

        // Clicking out of an input saves it, like leaving a field in a form
        if self.is_editing_inline_input() {
            self.commit_inline_input(terminal).await;
        }

        let Some((region, target)) = target else {
            return;
        };

        self.mouse.drag_origin = Some(DragOrigin { position, region, target, into_editor: false });

        self.click(target, region, position, is_double_click, event, terminal).await;
    }

    async fn click(&mut self, target: MouseTarget, region: Rect, position: Position, is_double_click: bool, event: MouseEvent, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        let key_bindings = *KEY_BINDINGS.read();

        match target {
            MouseTarget::Text | MouseTarget::Popup | MouseTarget::Help | MouseTarget::ResultContent | MouseTarget::LogsContent | MouseTarget::ExportContent => {},
            MouseTarget::Backdrop => self.press(key_bindings.generic.navigation.go_back, terminal).await,
            MouseTarget::PopupChoice(index) => {
                match self.state {
                    AppState::ChoosingElementToCreate => self.creation_popup.selection = index,
                    AppState::ChoosingRequestExportFormat => self.export_request.selection = index,
                    AppState::DeletingCollection => self.delete_collection_popup.state = index == 1,
                    AppState::DeletingRequest => self.delete_request_popup.state = index == 1,
                    _ => return
                }

                self.press(key_bindings.generic.navigation.select, terminal).await;
            },
            MouseTarget::SettingsRow(index) => {
                let was_selected = self.request_settings_popup.selection == index;
                self.request_settings_popup.selection = index;

                if was_selected {
                    self.press(key_bindings.generic.navigation.move_cursor_right, terminal).await;
                }
            },
            MouseTarget::Environment => self.press(key_bindings.main_menu.next_environment, terminal).await,
            MouseTarget::Collections => self.click_collections(position),
            MouseTarget::FooterKey(key) => self.press(key, terminal).await,

            /* Request */

            MouseTarget::SendButton => {
                self.focus_request();
                self.press(key_bindings.request_selected.send_request, terminal).await;
            },
            MouseTarget::Method => {
                self.focus_request();
                self.press(key_bindings.request_selected.change_method, terminal).await;
            },
            MouseTarget::Url => {
                self.focus_request();
                self.edit_request_url_state();
                self.forward_to_active_input(position, event);
            },
            MouseTarget::ParamTab(tab) => {
                self.focus_request();
                self.request_param_tab = tab;
                self.tui_load_a_request_param_tab();
            },
            MouseTarget::ChangeType => {
                self.focus_request();

                let key = match self.request_param_tab {
                    RequestParamsTabs::Auth => key_bindings.request_selected.param_tabs.change_auth_method,
                    RequestParamsTabs::Body => key_bindings.request_selected.param_tabs.change_body_content_type,
                    _ => key_bindings.request_selected.param_tabs.change_message_type,
                };

                self.press(key, terminal).await;
            },
            MouseTarget::Table(table_id) => {
                if table_id != TableId::EnvEditor {
                    self.focus_request();
                }

                self.click_table(table_id, position, is_double_click, terminal).await;
                self.forward_to_active_input(position, event);
            },
            MouseTarget::AuthField(index) => {
                self.focus_request();
                self.auth_text_input_selection.selected = index;
                self.press(key_bindings.generic.list_and_table_actions.edit_element, terminal).await;

                match self.state {
                    // Not a text field, cycle through its values
                    AppState::SelectedRequest => self.press(key_bindings.generic.navigation.move_cursor_right, terminal).await,
                    _ => self.forward_to_active_input(position, event)
                }
            },
            MouseTarget::MqttField(index) => {
                self.focus_request();
                self.mqtt_form_selection.selected = index;
                self.press(key_bindings.generic.list_and_table_actions.edit_element, terminal).await;
                self.forward_to_active_input(position, event);
            },
            MouseTarget::BodyFile | MouseTarget::BodyText => {
                self.focus_request();
                self.edit_request_body_file_or_string_state();
                self.forward_to_active_input(position, event);
            },
            MouseTarget::Message => {
                self.focus_request();
                self.edit_request_message_state();
                self.forward_to_active_input(position, event);
            },
            MouseTarget::Script(index) => {
                self.focus_request();
                self.script_console.script_selection = index;
                self.edit_request_script_state();
                self.forward_to_active_input(position, event);
            },

            /* Result */

            MouseTarget::ResultTab(tab) => {
                self.focus_request();

                if is_double_click {
                    self.request_view = match self.request_view {
                        RequestView::OnlyResult => RequestView::Normal,
                        _ => RequestView::OnlyResult
                    };
                }

                if self.request_result_tab != tab {
                    self.request_result_tab = tab;
                    *self.received_response.lock() = true;
                }
            },
            MouseTarget::ResultVerticalScrollbar => scroll_to_mouse(&mut self.result_vertical_scrollbar, region.y, region.height, position.y),
            MouseTarget::ResultHorizontalScrollbar => scroll_to_mouse(&mut self.result_horizontal_scrollbar, region.x, region.width, position.x),
        }
    }

    fn mouse_drag(&mut self, position: Position, event: MouseEvent) {
        let Some(origin) = self.mouse.drag_origin else {
            return;
        };

        if origin.into_editor {
            if let Some(input) = self.active_text_input() {
                input.mouse_event(event);
            }
            return;
        }

        match origin.target {
            MouseTarget::Backdrop => {},
            MouseTarget::ResultVerticalScrollbar => scroll_to_mouse(&mut self.result_vertical_scrollbar, origin.region.y, origin.region.height, position.y),
            MouseTarget::ResultHorizontalScrollbar => scroll_to_mouse(&mut self.result_horizontal_scrollbar, origin.region.x, origin.region.width, position.x),
            MouseTarget::ResultContent => {
                let area = self.last_result_area;

                // Scroll when dragging out of the result
                if position.y < area.top() {
                    self.result_vertical_scrollbar.scroll_by(-1);
                }
                else if position.y >= area.bottom() {
                    self.result_vertical_scrollbar.scroll_by(1);
                }

                if position.x < area.left() {
                    self.result_horizontal_scrollbar.scroll_by(-1);
                }
                else if position.x >= area.right() {
                    self.result_horizontal_scrollbar.scroll_by(1);
                }

                let head = self.result_content_position(position);

                match &mut self.selection {
                    Some(selection) if selection.kind == SelectionKind::ResultContent => selection.head = head,
                    _ => {
                        let anchor = self.result_content_position(origin.position);

                        self.selection = Some(Selection {
                            kind: SelectionKind::ResultContent,
                            region: area,
                            anchor,
                            head,
                        });
                    }
                }
            },
            _ => {
                let region = origin.region;
                let x = position.x.clamp(region.left(), region.right().saturating_sub(1));
                let y = position.y.clamp(region.top(), region.bottom().saturating_sub(1));

                self.selection = Some(Selection {
                    kind: SelectionKind::Screen,
                    region,
                    anchor: (origin.position.y as usize, origin.position.x as usize),
                    head: (y as usize, x as usize),
                });
            }
        }
    }

    fn mouse_up(&mut self, event: MouseEvent) {
        if let Some(origin) = self.mouse.drag_origin.take() && origin.into_editor && let Some(input) = self.active_text_input() {
            input.mouse_event(event);

            if input.selected_text().is_some() {
                self.toast(format!("{} to copy", KEY_BINDINGS.read().generic.copy));
            }
            return;
        }

        match &self.selection {
            Some(selection) if selection.is_empty() => self.selection = None,
            Some(_) => self.toast(format!("{} or right click to copy", KEY_BINDINGS.read().generic.copy)),
            None => {}
        }
    }

    fn scroll_at(&mut self, position: Position, delta: isize, horizontal: bool, event: MouseEvent) {
        if self.should_display_help {
            self.help_scroll = self.help_scroll.saturating_add_signed(delta);
            return;
        }

        let Some((_, target)) = self.target_at(position) else {
            return;
        };

        match target {
            MouseTarget::ResultContent | MouseTarget::ResultVerticalScrollbar | MouseTarget::ResultHorizontalScrollbar | MouseTarget::Text if self.last_result_area.contains(position) || target != MouseTarget::Text => {
                match horizontal {
                    true => self.result_horizontal_scrollbar.scroll_by(delta),
                    false => self.result_vertical_scrollbar.scroll_by(delta)
                }

                // Keep the selection under the mouse while scrolling
                if let Some(selection) = &self.selection && selection.kind == SelectionKind::ResultContent && self.mouse.drag_origin.is_some() {
                    let head = self.result_content_position(position);
                    self.selection.as_mut().unwrap().head = head;
                }
            },
            MouseTarget::Collections => {
                match delta < 0 {
                    true => self.collections_tree.state.scroll_up(delta.unsigned_abs()),
                    false => self.collections_tree.state.scroll_down(delta.unsigned_abs()),
                };
            },
            MouseTarget::Table(table_id) => {
                let table = self.table_mut(table_id);

                match delta < 0 {
                    true => table.up(),
                    false => table.down()
                }
            },
            MouseTarget::LogsContent => match horizontal {
                true => self.logs_horizontal_scrollbar.scroll_by(delta),
                false => self.logs_vertical_scrollbar.scroll_by(delta)
            },
            MouseTarget::ExportContent => match horizontal {
                true => self.display_request_export.horizontal_scrollbar.scroll_by(delta),
                false => self.display_request_export.vertical_scrollbar.scroll_by(delta)
            },
            MouseTarget::BodyText | MouseTarget::Message | MouseTarget::Script(_) | MouseTarget::AuthField(_) => {
                // Multi-line editors scroll themselves
                for input in self.multi_line_inputs() {
                    if input.last_area.contains(position) {
                        input.mouse_event(event);
                    }
                }
            },
            _ => {}
        }
    }

    /// Copy the selection of the input being edited, the mouse selection, or the link under the mouse.
    /// Returns false when there was nothing to copy.
    pub fn copy_selection_or_link(&mut self) -> bool {
        if let Some(input) = self.active_text_input() && let Some(text) = input.selected_text() {
            input.clear_selection();
            self.copy_to_clipboard(&text, "Copied");
            return true;
        }

        if let Some(selection) = self.selection {
            let text = match selection.kind {
                SelectionKind::Screen => self.selection_text.clone().unwrap_or_default(),
                SelectionKind::ResultContent => {
                    let (start, end) = selection.ordered();
                    self.result_view.text_between(start, end)
                }
            };

            self.copy_to_clipboard(&text, "Copied");
            return true;
        }

        if let Some(link) = self.hovered_link.clone() {
            self.copy_to_clipboard(&link, "Link copied");
            return true;
        }

        false
    }

    /// Paste text from the terminal (bracketed paste) into the input being edited
    pub fn handle_paste(&mut self, text: String) {
        if let Some(input) = self.active_text_input() {
            input.paste(text);
        }
    }

    /// Simulate a key press, used by mouse actions to reuse the keyboard logic
    async fn press(&mut self, key: KeyCombination, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        self.update_current_available_events();
        self.handle_key(key, terminal).await;
        self.update_current_available_events();
    }

    /// The request panel was clicked while the collections had the focus
    fn focus_request(&mut self) {
        if self.collections_tree.selected.is_some() && self.state == AppState::Normal {
            self.select_request_state();
        }
    }

    fn click_collections(&mut self, position: Position) {
        let selected_before = self.collections_tree.state.selected().to_vec();

        if !self.collections_tree.state.click_at(position) {
            return;
        }

        let selected = self.collections_tree.state.selected().to_vec();

        match selected.len() {
            // A collection was clicked for the first time, open it
            1 if selected != selected_before => {
                self.collections_tree.state.toggle_selected();
            },
            2 => self.select_request(),
            _ => {}
        }
    }

    async fn click_table(&mut self, table_id: TableId, position: Position, is_double_click: bool, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        let table = self.table_mut(table_id);

        let Some(cell) = table.cell_at(position) else {
            return;
        };

        if table.selection != Some(cell) && !is_double_click {
            table.update_selection(Some(cell));
            return;
        }

        table.update_selection(Some(cell));

        // Clicking the selected cell edits it
        match table_id {
            TableId::QueryParams => self.edit_request_param_state(),
            TableId::Headers => self.edit_request_header_state(),
            TableId::BodyForm => self.edit_request_body_table_state(),
            TableId::EnvEditor => self.edit_env_variable_state(),
            TableId::MqttSubscriptions => {
                let key = KEY_BINDINGS.read().generic.list_and_table_actions.edit_element;
                self.press(key, terminal).await;
            }
        }
    }

    fn forward_to_active_input(&mut self, position: Position, event: MouseEvent) {
        if let Some(input) = self.active_text_input() && input.last_area.contains(position) {
            let region = input.last_area;
            input.mouse_event(event);

            if let Some(origin) = &mut self.mouse.drag_origin {
                origin.region = region;
                origin.into_editor = true;
            }
        }
    }

    /// Position in the result content of a screen position
    fn result_content_position(&self, position: Position) -> (usize, usize) {
        let area = self.last_result_area;

        let x = position.x.clamp(area.left(), area.right().saturating_sub(1));
        let y = position.y.clamp(area.top(), area.bottom().saturating_sub(1));

        (
            self.result_vertical_scrollbar.scroll + (y - area.y) as usize,
            self.result_horizontal_scrollbar.scroll + (x - area.x) as usize,
        )
    }

    /// Inputs edited in place in the request panel (as opposed to popups)
    fn is_editing_inline_input(&self) -> bool {
        use AppState::*;

        matches!(
            self.state,
            EditingRequestUrl | EditingRequestParam |
            EditingRequestAuthBasicUsername | EditingRequestAuthBasicPassword | EditingRequestAuthBearerToken |
            EditingRequestAuthJwtSecret | EditingRequestAuthJwtPayload |
            EditingRequestAuthDigestUsername | EditingRequestAuthDigestPassword | EditingRequestAuthDigestDomains |
            EditingRequestAuthDigestRealm | EditingRequestAuthDigestNonce | EditingRequestAuthDigestOpaque |
            EditingRequestHeader | EditingRequestBodyTable | EditingRequestBodyFile | EditingRequestBodyString |
            EditingRequestMessage | EditingMqttFormField | EditingMqttSubscription |
            EditingPreRequestScript | EditingPostRequestScript | EditingEnvVariable
        )
    }

    /// Save the input being edited, as if the confirm key was pressed
    async fn commit_inline_input(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        // Confirming a message sends it, only leave the editor
        if self.state == AppState::EditingRequestMessage {
            self.state = AppState::SelectedRequest;
            return;
        }

        if let Some(input) = self.active_text_input() {
            input.clear_selection();
            input.reset_mode();
        }

        self.update_current_available_events();

        // The confirm event is always the first one of input states
        let confirm_key = AVAILABLE_EVENTS
            .read()
            .first()
            .and_then(|event| event.get_event_key_bindings().keys.first().copied());

        if let Some(confirm_key) = confirm_key {
            self.handle_key(confirm_key, terminal).await;
        }
    }

    /// The text input currently being edited
    pub fn active_text_input(&mut self) -> Option<&mut TextInput> {
        use AppState::*;

        let input = match self.state {
            EditingEnvVariable => &mut self.env_editor_table.selection_text_input,
            CreatingNewCollection => &mut self.new_collection_input,
            CreatingNewRequest => &mut self.new_request_popup.text_input,
            RenamingCollection => &mut self.rename_collection_input,
            RenamingRequest => &mut self.rename_request_input,
            EditingRequestUrl => &mut self.url_text_input,
            EditingRequestParam => &mut self.query_params_table.selection_text_input,
            EditingRequestAuthBasicUsername => &mut self.auth_basic_username_text_input,
            EditingRequestAuthBasicPassword => &mut self.auth_basic_password_text_input,
            EditingRequestAuthBearerToken => &mut self.auth_bearer_token_text_input,
            EditingRequestAuthJwtSecret => &mut self.auth_jwt_secret_text_input,
            EditingRequestAuthJwtPayload => &mut self.auth_jwt_payload_text_area,
            EditingRequestAuthDigestUsername => &mut self.auth_digest_username_text_input,
            EditingRequestAuthDigestPassword => &mut self.auth_digest_password_text_input,
            EditingRequestAuthDigestDomains => &mut self.auth_digest_domains_text_input,
            EditingRequestAuthDigestRealm => &mut self.auth_digest_realm_text_input,
            EditingRequestAuthDigestNonce => &mut self.auth_digest_nonce_text_input,
            EditingRequestAuthDigestOpaque => &mut self.auth_digest_opaque_text_input,
            EditingRequestHeader => &mut self.headers_table.selection_text_input,
            EditingRequestBodyTable => &mut self.body_form_table.selection_text_input,
            EditingRequestBodyFile => &mut self.body_file_text_input,
            EditingRequestBodyString => &mut self.body_text_area,
            EditingRequestMessage => &mut self.message_text_area,
            EditingMqttFormField => &mut self.mqtt_form_text_input,
            EditingMqttSubscription => &mut self.mqtt_subscriptions_table.selection_text_input,
            EditingPreRequestScript => &mut self.script_console.pre_request_text_area,
            EditingPostRequestScript => &mut self.script_console.post_request_text_area,
            _ => return None
        };

        Some(input)
    }

    fn multi_line_inputs(&mut self) -> [&mut TextInput; 5] {
        [
            &mut self.body_text_area,
            &mut self.message_text_area,
            &mut self.auth_jwt_payload_text_area,
            &mut self.script_console.pre_request_text_area,
            &mut self.script_console.post_request_text_area,
        ]
    }
}

impl<'a> App<'a> {
    fn table_mut(&mut self, table_id: TableId) -> &mut StatefulCustomTable<'a> {
        match table_id {
            TableId::QueryParams => &mut self.query_params_table,
            TableId::Headers => &mut self.headers_table,
            TableId::BodyForm => &mut self.body_form_table,
            TableId::MqttSubscriptions => &mut self.mqtt_subscriptions_table,
            TableId::EnvEditor => &mut self.env_editor_table,
        }
    }
}

/// Jump a scrollbar to the mouse position along its track
fn scroll_to_mouse(scrollbar: &mut crate::tui::utils::stateful::stateful_scrollbar::StatefulScrollbar, track_start: u16, track_length: u16, mouse: u16) {
    let offset = mouse.saturating_sub(track_start) as usize;
    let track_length = (track_length as usize).saturating_sub(1).max(1);

    scrollbar.set_scroll(offset.min(track_length) * scrollbar.max_scroll / track_length);
}
