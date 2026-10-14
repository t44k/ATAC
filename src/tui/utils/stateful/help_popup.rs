use crate::tui::app_states::AppState;

#[derive(Default)]
pub struct HelpPopup {
    /// State whose keys are displayed
    pub selection: AppState,
}
