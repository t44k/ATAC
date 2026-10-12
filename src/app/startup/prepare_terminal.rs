use std::io::stdout;
use ratatui::crossterm::execute;
use ratatui::crossterm::event::{EnableBracketedPaste, EnableMouseCapture};
use ratatui::crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
use tracing::trace;
use crate::app::app::App;

impl App<'_> {
    pub fn prepare_terminal(&mut self) -> &mut Self {
        trace!("Preparing terminal...");
        
        enable_raw_mode().unwrap();
        execute!(stdout(), EnterAlternateScreen).unwrap();

        execute!(stdout(), EnableMouseCapture, EnableBracketedPaste).unwrap();

        trace!("Terminal OK");

        self
    }
}
