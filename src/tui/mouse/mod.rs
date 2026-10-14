//! Mouse support: every frame, render functions register the areas they draw into a hit map.
//! Mouse events are then resolved against the hit map of the last rendered frame.

pub mod handle;
pub mod selection;
pub mod tabs;

use std::time::Instant;

use crokey::KeyCombination;
use ratatui::layout::{Position, Rect};

use crate::app::app::App;
use crate::tui::ui::param_tabs::param_tabs::RequestParamsTabs;
use crate::tui::ui::result_tabs::RequestResultTabs;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableId {
    QueryParams,
    Headers,
    BodyForm,
    MqttSubscriptions,
    EnvEditor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseTarget {
    /// Any area that only supports text selection
    Text,
    /// Covers the whole screen below an open popup, clicking it closes the popup
    Backdrop,
    /// Popup body
    Popup,
    PopupChoice(usize),
    Environment,
    Collections,
    SendButton,
    Method,
    Url,
    ParamTab(RequestParamsTabs),
    /// Placeholder like "No auth", clicking it changes the type
    ChangeType,
    Table(TableId),
    AuthField(usize),
    MqttField(usize),
    BodyFile,
    BodyText,
    Message,
    Script(u16),
    ResultTab(RequestResultTabs),
    /// Text content of the result (body, headers, cookies, console)
    ResultContent,
    ResultVerticalScrollbar,
    ResultHorizontalScrollbar,
    LogsContent,
    ExportContent,
    SettingsRow(usize),
    FooterKey(KeyCombination),
    Help,
}

/// Where the left button was pressed, used for drags
#[derive(Debug, Clone, Copy)]
pub struct DragOrigin {
    pub position: Position,
    pub region: Rect,
    pub target: MouseTarget,
    /// The drag is handled by the text editor that was clicked
    pub into_editor: bool,
}

#[derive(Default)]
pub struct MouseState {
    pub position: Option<Position>,
    pub last_click: Option<(Instant, Position)>,
    pub drag_origin: Option<DragOrigin>,
}

impl App<'_> {
    /// Register an area drawn this frame. Areas registered later are on top.
    pub fn hit(&mut self, area: Rect, target: MouseTarget) {
        if area.width > 0 && area.height > 0 {
            self.hit_map.push((area, target));
        }
    }

    /// Topmost target drawn at the given position on the last frame
    pub fn target_at(&self, position: Position) -> Option<(Rect, MouseTarget)> {
        self.hit_map
            .iter()
            .rev()
            .find(|(area, _)| area.contains(position))
            .copied()
    }
}
