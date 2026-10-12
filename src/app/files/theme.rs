use std::fs;
use lazy_static::lazy_static;
use nestify::nest;
use parking_lot::RwLock;
use ratatui::style::Color;
use tracing::{info, trace};
use serde::{Deserialize, Serialize};

use crate::app::app::App;
use crate::app::files::utils::{find_config_file, parse_partial_toml};
use crate::panic_error;

nest! {
    #[derive(Serialize, Deserialize)]
    pub struct ThemeWebsocket {
        #[serde(alias = "Connection Statuses")]
        pub connection_status:
            #[derive(Serialize, Deserialize)]
            pub struct ThemeConnectionStatuses {
                #[serde(alias = "Connected")]
                pub connected: Color,
                #[serde(alias = "Disconnected")]
                pub disconnected: Color,
            },

        #[serde(alias = "Messages")]
        pub messages:
            #[derive(Serialize, Deserialize)]
            pub struct ThemeMessages {
                #[serde(alias = "server_foreground_color")]
                pub server_foreground_color: Color,
                #[serde(alias = "server_background_color")]
                pub server_background_color: Color,
                #[serde(alias = "you_background_color")]
                pub you_background_color: Color,
                #[serde(alias = "details_color")]
                pub details_color: Color,
            }
    }
}

nest! {
    #[derive(Serialize, Deserialize)]
    pub struct Theme {
        #[serde(alias = "UI")]
        pub ui:
            #[derive(Serialize, Deserialize)]
            pub struct ThemeUI {
                pub font_color: Color,
                pub app_background: Option<Color>,

                pub main_foreground_color: Color,
                pub secondary_foreground_color: Color,

                pub main_background_color: Color,
                pub secondary_background_color: Color
            },

        #[serde(alias = "Others")]
        pub others:
            #[derive(Serialize, Deserialize)]
            pub struct ThemeOthers {
                pub selection_highlight_color: Color,
                pub environment_variable_highlight_color: Color,
                /// Links hovered with the mouse in the result panel
                pub link_color: Color,
            },

        #[serde(alias = "HTTP")]
        pub http: #[derive(Serialize, Deserialize)]
            pub struct ThemeHttp {
            #[serde(alias = "Methods")]
            pub methods:
                #[derive(Serialize, Deserialize)]
                pub struct ThemeMethods {
                    #[serde(alias = "GET")]
                    pub get: Color,
                    #[serde(alias = "POST")]
                    pub post: Color,
                    #[serde(alias = "QUERY")]
                    pub query: Color,
                    #[serde(alias = "PUT")]
                    pub put: Color,
                    #[serde(alias = "PATCH")]
                    pub patch: Color,
                    #[serde(alias = "DELETE")]
                    pub delete: Color,
                    #[serde(alias = "HEAD")]
                    pub head: Color,
                    #[serde(alias = "OPTIONS")]
                    pub options: Color,
                    #[serde(alias = "TRACE")]
                    pub trace: Color,
                    #[serde(alias = "CONNECT")]
                    pub connect: Color
                },
        },

        #[serde(alias = "Websocket")]
        pub websocket: ThemeWebsocket,

        #[serde(alias = "MQTT")]
        pub mqtt: ThemeWebsocket
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            ui: ThemeUI {
                font_color: Color::White,
                app_background: None,

                main_foreground_color: Color::White,
                secondary_foreground_color: Color::DarkGray,

                main_background_color: Color::DarkGray,
                secondary_background_color: Color::Rgb(50, 50, 50),
            },
            others: ThemeOthers {
                selection_highlight_color: Color::Yellow,
                environment_variable_highlight_color: Color::Cyan,
                link_color: Color::LightBlue,
            },
            http: ThemeHttp {
                methods: ThemeMethods {
                    get: Color::Green,
                    post: Color::Yellow,
                    query: Color::LightGreen,
                    put: Color::LightBlue,
                    patch: Color::LightCyan,
                    delete: Color::LightRed,
                    options: Color::Magenta,
                    head: Color::Green,
                    trace: Color::Yellow,
                    connect: Color::LightBlue
                },
            },
            websocket: ThemeWebsocket {
                connection_status: ThemeConnectionStatuses {
                    connected: Color::Green,
                    disconnected: Color::Red
                },

                messages: ThemeMessages {
                    server_foreground_color: Color::Blue,
                    server_background_color: Color::DarkGray,
                    you_background_color: Color::Blue,
                    details_color: Color::DarkGray
                }
            },
            mqtt: ThemeWebsocket {
                connection_status: ThemeConnectionStatuses {
                    connected: Color::Green,
                    disconnected: Color::Red
                },

                messages: ThemeMessages {
                    server_foreground_color: Color::Blue,
                    server_background_color: Color::DarkGray,
                    you_background_color: Color::Blue,
                    details_color: Color::DarkGray
                }
            }
        }
    }
}

lazy_static! {
    pub static ref THEME: RwLock<Theme> = RwLock::new(Theme::default());
}

impl App<'_> {
    /// Load the theme from `ATAC_THEME`, or `theme.toml` in the app or config directory.
    /// The file can only contain the colors to change.
    pub fn parse_theme_file(&mut self) {
        let path = match find_config_file("ATAC_THEME", &["theme.toml"]) {
            Some(path) => path,
            None => {
                trace!("No theme file found, using default");
                return;
            }
        };

        trace!("Parsing theme file \"{}\"", path.display());

        let file_content = match fs::read_to_string(&path) {
            Ok(file_content) => file_content,
            Err(e) => panic_error(format!("Could not open theme file \"{}\"\n\t{e}", path.display()))
        };

        let theme: Theme = match parse_partial_toml(&Theme::default(), &file_content) {
            Ok(theme) => theme,
            Err(e) => panic_error(format!("Could not parse theme file \"{}\"\n\t{e}", path.display()))
        };

        *THEME.write() = theme;

        info!("Theme loaded from \"{}\"", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_theme_keeps_default_colors() {
        let theme = parse_partial_toml(&Theme::default(), "[others]\nlink_color = \"Magenta\"").unwrap();

        assert_eq!(theme.others.link_color, Color::Magenta);
        assert_eq!(theme.others.selection_highlight_color, Color::Yellow);
        assert_eq!(theme.http.methods.get, Color::Green);
    }

    #[test]
    fn example_themes_still_parse() {
        for entry in fs::read_dir("example_resources/themes").unwrap() {
            let path = entry.unwrap().path();
            let content = fs::read_to_string(&path).unwrap();

            parse_partial_toml(&Theme::default(), &content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
}
