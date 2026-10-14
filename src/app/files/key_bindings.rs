use std::fs;

use tracing::{info, trace};
use crokey::{key, KeyCombination};
use lazy_static::lazy_static;
use nestify::nest;
use parking_lot::RwLock;
use ratatui::text::Span;
use serde::{Deserialize, Serialize};

use crate::app::app::App;
use crate::app::files::utils::{find_config_file, parse_partial_toml};
use crate::panic_error;

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct KeyBindingsConfig {
    pub keybindings: KeyBindings,
}

nest! {
    #[derive(Copy, Clone, Serialize, Deserialize)]
    pub struct KeyBindings {
        pub main_menu: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct MainMenu {
            pub exit: KeyCombination,
            pub alt_exit: KeyCombination,

            pub expand_collection: KeyCombination,
            pub unselect_request: KeyCombination,

            pub move_request_up: KeyCombination,
            pub move_request_down: KeyCombination,

            pub next_environment: KeyCombination,

            pub display_env_editor: KeyCombination,
            pub display_cookies: KeyCombination,
            pub display_logs: KeyCombination,
        },

        pub generic: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct Generic {
            pub display_help: KeyCombination,
            pub alt_display_help: KeyCombination,

            /// Copy the mouse selection, or the link under the mouse
            pub copy: KeyCombination,

            pub text_input: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct TextInput {
                pub quit_without_saving: KeyCombination,
                pub save_and_quit_single_line: KeyCombination,
                pub save_and_quit_area: KeyCombination,
                pub mode: #[derive(Copy, Clone, PartialEq, Serialize, Deserialize)] pub enum TextAreaMode {
                    #[serde(alias = "vim", alias = "VIM")]
                    Vim,
                    #[serde(alias = "emacs", alias = "EMACS")]
                    Emacs,
                    #[serde(alias = "default", alias = "DEFAULT")]
                    Default,
                    #[serde(alias = "custom", alias = "CUSTOM")]
                    Custom(CustomTextArea)
                },
            },

            /// Navigation in tables, popups, up and down in the collections list
            pub navigation: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct Navigation {
                pub move_cursor_up: KeyCombination,
                pub move_cursor_down: KeyCombination,
                pub move_cursor_left: KeyCombination,
                pub move_cursor_right: KeyCombination,

                pub alt_move_cursor_up: KeyCombination,
                pub alt_move_cursor_down: KeyCombination,
                #[allow(dead_code)]
                pub alt_move_cursor_left: KeyCombination,
                #[allow(dead_code)]
                pub alt_move_cursor_right: KeyCombination,

                pub go_back: KeyCombination,
                pub select: KeyCombination,
            },

            pub list_and_table_actions: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct ListAndTableActions {
                pub create_element: KeyCombination,
                pub delete_element: KeyCombination,
                pub alt_delete_element: KeyCombination,
                pub edit_element: KeyCombination,
                /// Only used in the collections list (main menu)
                pub rename_element: KeyCombination,
                pub alt_rename_element: KeyCombination,
                /// Only used in tables (Query params, headers, cookies)
                pub toggle_element: KeyCombination,
                pub duplicate_element: KeyCombination,
            }
        },

        pub request_selected: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct RequestSelected {
            pub param_next_tab: KeyCombination,
            pub param_previous_tab: KeyCombination,
            pub change_url: KeyCombination,
            pub alt_change_url: KeyCombination,
            pub change_method: KeyCombination,
            pub request_settings: KeyCombination,
            pub export_request: KeyCombination,

            pub next_view: KeyCombination,

            pub send_request: KeyCombination,
            pub alt_send_request: KeyCombination,

            pub param_tabs: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct ParamTabs {
                pub change_auth_method: KeyCombination,
                pub change_body_content_type: KeyCombination,
                pub change_message_type: KeyCombination,
            },

            pub result_tabs: #[derive(Copy, Clone, Serialize, Deserialize)] pub struct ResultTabs {
                pub scroll_up: KeyCombination,
                pub scroll_down: KeyCombination,
                pub scroll_left: KeyCombination,
                pub scroll_right: KeyCombination,
                pub scroll_page_up: KeyCombination,
                pub scroll_page_down: KeyCombination,
                pub scroll_to_top: KeyCombination,
                pub scroll_to_bottom: KeyCombination,

                pub yank_response_part: KeyCombination,

                /// Will use param_next_tab depending on the selected view
                pub result_next_tab: KeyCombination,
                pub result_previous_tab: KeyCombination,
            }
        },
    }
}

lazy_static! {
    pub static ref KEY_BINDINGS: RwLock<KeyBindings> = RwLock::new(KeyBindings::default());
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTextArea {
    pub copy: KeyCombination,
    pub cut: KeyCombination,
    pub paste: KeyCombination,
    pub select_all: KeyCombination,

    pub search: KeyCombination,
    pub system_editor: KeyCombination,

    pub undo: KeyCombination,
    pub redo: KeyCombination,

    pub new_line: KeyCombination,
    pub indent: KeyCombination,

    pub delete_backward: KeyCombination,
    pub delete_forward: KeyCombination,

    pub skip_word_right: KeyCombination,
    pub skip_word_left: KeyCombination,

    pub move_cursor_up: KeyCombination,
    pub move_cursor_down: KeyCombination,
    pub move_cursor_left: KeyCombination,
    pub move_cursor_right: KeyCombination,
    pub move_cursor_line_start: KeyCombination,
    pub move_cursor_line_end: KeyCombination,
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            main_menu: MainMenu {
                exit: key!(q),
                alt_exit: key!(ctrl-q),

                expand_collection: key!(right),
                unselect_request: key!(left),

                move_request_up: key!(ctrl-up),
                move_request_down: key!(ctrl-down),

                next_environment: key!(e),

                display_env_editor: key!(ctrl-e),
                display_cookies: key!(c),
                display_logs: key!(l),
            },

            generic: Generic {
                display_help: key!('?'),
                alt_display_help: key!(f1),

                copy: key!(ctrl-c),

                text_input: TextInput {
                    quit_without_saving: key!(esc),
                    save_and_quit_single_line: key!(enter),
                    save_and_quit_area: key!(ctrl-s),
                    mode: TextAreaMode::Default,
                },

                navigation: Navigation {
                    move_cursor_up: key!(up),
                    move_cursor_down: key!(down),
                    move_cursor_left: key!(left),
                    move_cursor_right: key!(right),

                    alt_move_cursor_up: key!(Up),
                    alt_move_cursor_down: key!(Down),
                    alt_move_cursor_left: key!(Left),
                    alt_move_cursor_right: key!(Right),
                    
                    go_back: key!(esc),
                    select: key!(enter),
                },
                list_and_table_actions: ListAndTableActions {
                    create_element: key!(n),
                    delete_element: key!(d),
                    alt_delete_element: key!(delete),
                    edit_element: key!(enter),
                    rename_element: key!(r),
                    alt_rename_element: key!(f2),
                    toggle_element: key!(t),
                    duplicate_element: key!(ctrl-d),
                },
            },

            request_selected: RequestSelected {
                param_next_tab: key!(tab),
                param_previous_tab: key!(shift-backtab),

                change_url: key!(ctrl-l),
                alt_change_url: key!(u),
                change_method: key!(m),

                request_settings: key!(s),
                export_request: key!(shift-E),
                
                next_view: key!(v),

                // Ctrl + enter doesn't register right on many platforms, F5 always does
                // https://github.com/crossterm-rs/crossterm/issues/685
                send_request: key!(space),
                alt_send_request: key!(f5),

                // Only active on their own tab, so they can share the same key
                param_tabs: ParamTabs {
                    change_auth_method: key!(ctrl-t),
                    change_body_content_type: key!(ctrl-t),
                    change_message_type: key!(ctrl-t),
                },
                result_tabs: ResultTabs {
                    scroll_up: key!(ctrl-up),
                    scroll_down: key!(ctrl-down),
                    scroll_left: key!(ctrl-left),
                    scroll_right: key!(ctrl-right),
                    scroll_page_up: key!(pageup),
                    scroll_page_down: key!(pagedown),
                    scroll_to_top: key!(home),
                    scroll_to_bottom: key!(end),

                    yank_response_part: key!(y),

                    result_next_tab: key!(']'),
                    result_previous_tab: key!('['),
                },
            }
        }
    }
}

impl Default for CustomTextArea {
    fn default() -> Self {
        CustomTextArea {
            copy: key!(ctrl-c),
            cut: key!(ctrl-x),
            paste: key!(ctrl-v),
            select_all: key!(ctrl-a),

            search: key!(ctrl-f),
            system_editor: key!(ctrl-e),

            undo: key!(ctrl-z),
            redo: key!(ctrl-y),

            new_line: key!(enter),
            indent: key!(tab),

            delete_backward: key!(delete),
            delete_forward: key!(backspace),

            skip_word_right: key!(ctrl-right),
            skip_word_left: key!(ctrl-left),

            move_cursor_up: key!(up),
            move_cursor_down: key!(down),
            move_cursor_left: key!(left),
            move_cursor_right: key!(right),
            move_cursor_line_start: key!(home),
            move_cursor_line_end: key!(end),
        }
    }
}

impl App<'_> {
    /// Load the key bindings from `ATAC_KEY_BINDINGS`, or `keybindings.toml` in the app or config directory.
    /// The file can only contain the bindings to change.
    pub fn parse_key_bindings_file(&mut self) {
        let path = match find_config_file("ATAC_KEY_BINDINGS", &["keybindings.toml", "key_bindings.toml"]) {
            Some(path) => path,
            None => {
                trace!("No key bindings file found, using default");
                return;
            }
        };

        trace!("Parsing key bindings file \"{}\"", path.display());

        let file_content = match fs::read_to_string(&path) {
            Ok(file_content) => file_content,
            Err(e) => panic_error(format!("Could not open key bindings file \"{}\"\n\t{e}", path.display()))
        };

        let config: KeyBindingsConfig = match parse_partial_toml(&KeyBindingsConfig::default(), &file_content) {
            Ok(config) => config,
            Err(e) => panic_error(format!("Could not parse key bindings file \"{}\"\n\t{e}", path.display()))
        };

        *KEY_BINDINGS.write() = config.keybindings;

        info!("Key bindings loaded from \"{}\"", path.display());
    }
}

pub fn unique_key_and_help(help: Span<'static>, key: Span<'static>) -> Vec<Span<'static>> {
    if help.to_string() == key.to_string() {
        return vec![help];
    }
    else {
        return vec![help, Span::raw(" "), key]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_key_bindings_keep_defaults() {
        let config = parse_partial_toml(&KeyBindingsConfig::default(), "[keybindings.main_menu]\nexit = \"Ctrl-x\"").unwrap();

        assert_eq!(config.keybindings.main_menu.exit, key!(ctrl-x));
        assert_eq!(config.keybindings.generic.display_help, key!('?'));
        assert_eq!(config.keybindings.request_selected.send_request, key!(space));
    }

    #[test]
    fn partial_custom_text_area() {
        let config = parse_partial_toml(&KeyBindingsConfig::default(), "[keybindings.generic.text_input.mode.custom]\nundo = \"Ctrl-u\"").unwrap();

        match config.keybindings.generic.text_input.mode {
            TextAreaMode::Custom(custom) => {
                assert_eq!(custom.undo, key!(ctrl-u));
                assert_eq!(custom.copy, key!(ctrl-c));
            },
            _ => panic!("Expected custom mode")
        }
    }

    #[test]
    fn default_example_file_matches_defaults() {
        let content = fs::read_to_string("example_resources/key_bindings/default_key_bindings.toml").unwrap();
        let mut from_file: KeyBindingsConfig = toml::from_str(&content).unwrap();

        // The file spells out the default text input keys
        assert!(from_file.keybindings.generic.text_input.mode == TextAreaMode::Custom(CustomTextArea::default()));
        from_file.keybindings.generic.text_input.mode = TextAreaMode::Default;

        assert_eq!(toml::Value::try_from(from_file).unwrap(), toml::Value::try_from(KeyBindingsConfig::default()).unwrap());
    }

    #[test]
    fn example_key_bindings_still_parse() {
        for entry in fs::read_dir("example_resources/key_bindings").unwrap() {
            let path = entry.unwrap().path();
            let content = fs::read_to_string(&path).unwrap();

            parse_partial_toml(&KeyBindingsConfig::default(), &content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
}
