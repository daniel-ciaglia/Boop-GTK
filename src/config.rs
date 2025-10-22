use std::{fs::File, io::Write};

use crate::XDG_DIRS;
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};

fn default_font_family() -> String {
    String::from("monospace")
}

fn default_monospace_filter() -> bool {
    true
}

fn default_light_scheme() -> String {
    String::from("solarized-light")
}

fn default_dark_scheme() -> String {
    String::from("solarized-dark")
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub show_shortcuts_on_open: bool,
    pub editor: Editor,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Editor {
    pub colour_scheme_id: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_monospace_filter")]
    pub monospace_filter: bool,
    
    // System theme sync configuration
    #[serde(default)]
    pub sync_with_system_theme: bool,
    #[serde(default = "default_light_scheme")]
    pub light_scheme_id: String,
    #[serde(default = "default_dark_scheme")]
    pub dark_scheme_id: String,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            colour_scheme_id: String::from("classic"),
            font_family: String::from("monospace"),
            monospace_filter: true,
            sync_with_system_theme: false,
            light_scheme_id: String::from("solarized-light"),
            dark_scheme_id: String::from("solarized-dark"),
        }
    }
}

impl Config {
    pub fn load() -> Result<(Config, bool)> {
        let mut config_file_created = false;

        let config_path = XDG_DIRS
            .place_config_file("config.toml")
            .wrap_err("Failed to place config file")?;

        if !config_path.exists() {
            info!("creating config.toml");
            config_file_created = true;

            // no config file, write default
            let mut file = File::create(&config_path).wrap_err("Failed to create config file")?;
            file.write_all(
                toml::to_string_pretty(&Config::default())
                    .unwrap()
                    .as_bytes(),
            )
            .wrap_err("Failed to write to config file")?;
        }

        let settings = config::Config::builder().add_source(config::File::from(config_path));

        let config = settings
            .build()
            .wrap_err("Failed to covert settings into Config")?
            .try_deserialize()?;

        Ok((config, config_file_created)) // TODO: handle results
    }

    pub fn save(&self) -> Result<()> {
        let config_path = XDG_DIRS
            .place_config_file("config.toml")
            .wrap_err("Failed to place config file")?;

        File::create(config_path)
            .wrap_err("Failed to create config file")?
            .write_all(toml::to_string_pretty(self).unwrap().as_bytes())
            .wrap_err("Failed to write to config file")
    }

    pub fn set_show_shortcuts_on_open(&mut self, enable: bool) {
        self.show_shortcuts_on_open = enable;
    }
}

impl Editor {
    pub fn set_colour_scheme_id(&mut self, id: &str) {
        self.colour_scheme_id = String::from(id);
    }

    pub fn set_font_family(&mut self, font: &str) {
        self.font_family = String::from(font);
    }

    pub fn set_monospace_filter(&mut self, enabled: bool) {
        self.monospace_filter = enabled;
    }

    pub fn set_sync_with_system_theme(&mut self, enabled: bool) {
        self.sync_with_system_theme = enabled;
    }

    pub fn set_light_scheme_id(&mut self, id: &str) {
        self.light_scheme_id = String::from(id);
    }

    pub fn set_dark_scheme_id(&mut self, id: &str) {
        self.dark_scheme_id = String::from(id);
    }

    pub fn get_current_scheme_id(&self, is_dark_mode: bool) -> &str {
        if self.sync_with_system_theme {
            if is_dark_mode {
                &self.dark_scheme_id
            } else {
                &self.light_scheme_id
            }
        } else {
            &self.colour_scheme_id
        }
    }
}
