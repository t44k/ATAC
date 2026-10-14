//! System clipboard access shared by the whole app (result copy, selections, text inputs).
//!
//! Text goes to the system clipboard through arboard when available, and falls back to the
//! OSC 52 terminal escape sequence otherwise (SSH sessions, no display server, no clipboard feature).

use std::io::{stdout, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lazy_static::lazy_static;
use parking_lot::Mutex;
#[cfg(feature = "clipboard")]
use tracing::warn;

#[cfg(feature = "clipboard")]
lazy_static! {
    static ref SYSTEM_CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(arboard::Clipboard::new().ok());
}

lazy_static! {
    /// Last copied text, used as a fallback when the system clipboard cannot be read
    static ref LAST_COPIED: Mutex<String> = Mutex::new(String::new());

    /// When set, text copied by the text editors is not sent to the system clipboard (e.g. deleting a selection)
    static ref MUTED: Mutex<bool> = Mutex::new(false);
}

/// Copy text to the system clipboard, or through the terminal with OSC 52 when no clipboard is reachable
pub fn copy_text(text: &str) {
    *LAST_COPIED.lock() = text.to_string();

    #[cfg(feature = "clipboard")]
    if let Some(clipboard) = SYSTEM_CLIPBOARD.lock().as_mut() {
        match clipboard.set_text(text) {
            Ok(_) => return,
            Err(e) => warn!("Could not copy to the system clipboard, using OSC 52 instead: {e}")
        }
    }

    copy_with_osc52(text);
}

/// Read text from the system clipboard, or the last text copied by the app
pub fn get_text() -> String {
    #[cfg(feature = "clipboard")]
    if let Some(clipboard) = SYSTEM_CLIPBOARD.lock().as_mut() && let Ok(text) = clipboard.get_text() {
        return text;
    }

    LAST_COPIED.lock().clone()
}

#[cfg(feature = "clipboard")]
pub fn copy_image(image: arboard::ImageData) -> bool {
    match SYSTEM_CLIPBOARD.lock().as_mut() {
        Some(clipboard) => clipboard.set_image(image).is_ok(),
        None => false
    }
}

/// Run a closure while text editor copies are not forwarded to the system clipboard
pub fn muted<T>(f: impl FnOnce() -> T) -> T {
    *MUTED.lock() = true;
    let result = f();
    *MUTED.lock() = false;
    result
}

/// Hand the clipboard content over before exiting (needed on X11/Wayland where the app owns the content)
pub fn shutdown() {
    #[cfg(feature = "clipboard")]
    drop(SYSTEM_CLIPBOARD.lock().take());
}

fn copy_with_osc52(text: &str) {
    let sequence = format!("\x1b]52;c;{}\x07", STANDARD.encode(text));
    let mut stdout = stdout();
    let _ = stdout.write_all(sequence.as_bytes());
    let _ = stdout.flush();
}

/// Clipboard given to every edtui text input so that their copy/cut/paste use the system clipboard
pub struct EditorClipboard;

impl edtui::clipboard::ClipboardTrait for EditorClipboard {
    fn set_text(&mut self, text: String) {
        if *MUTED.lock() {
            return;
        }

        copy_text(&text);
    }

    fn get_text(&mut self) -> String {
        get_text()
    }
}
