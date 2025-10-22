use crate::{
    config::Config,
    executor::{self},
    script::Script,
    scriptmap::ScriptMap,
    ui::command_palette::Dialog as CommandDialog,
    ui::{preferences_dialog::Dialog as PreferencesDialog, shortcuts_window::ShortcutsWindow},
    util::SourceViewExt,
    util::StringExt,
    XDG_DIRS,
};
use eyre::{Context, Result};
use gdk_pixbuf::prelude::*;
use gladis::Gladis;
use glib::SourceId;
use gtk::{prelude::*, Label, Revealer, Settings};
use sourceview::{prelude::*, Language};

use executor::{ExecutorError, TextReplacement};
use gtk::{ApplicationWindow, Button, ModelButton};
use std::sync::{Arc, RwLock};

use super::about_dialog::AboutDialog;

pub const NOTIFICATION_LONG_DELAY: u32 = 5000;

#[derive(Gladis, Clone, Shrinkwrap)]
pub struct Widgets {
    #[shrinkwrap(main_field)]
    pub window: ApplicationWindow,

    header_button: Button,
    source_view: sourceview::View,
    // status_bar: Statusbar,
    notification_revealer: Revealer,
    notification_label: Label,
    notification_button: Button,

    re_execute_last_script_button: ModelButton,
    reset_scripts_button: ModelButton,
    preferences_button: ModelButton,
    config_directory_button: ModelButton,
    more_scripts_button: ModelButton,
    shortcuts_button: ModelButton,
    about_button: ModelButton,
}

#[derive(Clone, Shrinkwrap)]
pub struct App {
    #[shrinkwrap(main_field)]
    pub widgets: Widgets,
    preferences_dialog: PreferencesDialog,
    about_dialog: AboutDialog,

    scripts: Arc<RwLock<ScriptMap>>,
    notification_source_id: Arc<RwLock<Option<SourceId>>>,
    last_script_executed: Arc<RwLock<Option<String>>>,
    config: Arc<RwLock<Config>>,
}

impl App {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new(
        boop_language: &Language,
        scripts: Arc<RwLock<ScriptMap>>,
        config: Arc<RwLock<Config>>,
    ) -> Result<Self> {
        let app = App {
            widgets: Widgets::from_resource("/fyi/zoey/Boop-GTK/boop-gtk.glade")
                .wrap_err("Failed to load boop-gtk.glade")?,
            preferences_dialog: PreferencesDialog::new(&config)?,
            about_dialog: AboutDialog::new(&scripts)?,
            scripts,
            notification_source_id: Arc::new(RwLock::new(None)),
            last_script_executed: Arc::new(RwLock::new(None)),
            config,
        };

        app.configure(boop_language)?;
        app.update_state_from_config()?;
        
        // Setup system theme monitoring if enabled
        {
            let config = app.config.read().map_err(|e| eyre!("Config lock poisoned: {}", e))?;
            if config.editor.sync_with_system_theme {
                app.setup_system_theme_monitoring()?;
            }
        }

        // close notification on dismiss
        {
            let notification_revealer = app.notification_revealer.clone();
            app.notification_button
                .connect_button_press_event(move |_button, _event| {
                    notification_revealer.set_reveal_child(false);
                    Inhibit(false)
                });
        }

        // re-execute last script
        {
            let app_ = app.clone();
            app.re_execute_last_script_button
                .connect_clicked(move |_| app_.re_execute().expect("Failed to re-execute script"));
        }

        // reset the state of each script
        {
            let scripts = app.scripts.clone();
            app.reset_scripts_button.connect_clicked(move |_| {
                for script in scripts
                    .write()
                    .expect("Scripts lock is poisoned")
                    .0
                    .values_mut()
                {
                    script.kill_thread();
                }
            });
        }

        // open preferences dialog
        {
            let preference_dialog = app.preferences_dialog.clone();
            app.preferences_button.connect_clicked(move |_| {
                let responce = preference_dialog.run();
                if responce == gtk::ResponseType::DeleteEvent
                    || responce == gtk::ResponseType::Cancel
                {
                    preference_dialog.hide();
                }
            });
        }

        {
            let source_view: sourceview::View = app.source_view.clone();
            app.preferences_dialog
                .connect_config_style_scheme_notify(move |scheme| {
                    source_view
                        .get_sourceview_buffer()
                        .expect("Failed to get sourceview buffer")
                        .set_style_scheme(scheme.as_ref());
                });
        }

        {
            let source_view: sourceview::View = app.source_view.clone();
            app.preferences_dialog
                .connect_config_font_notify(move |font| {
                    let font_desc = pango::FontDescription::from_string(font);
                    source_view.override_font(&font_desc);
                });
        }

        // Connect system theme sync notifications
        {
            let app_ = app.clone();
            app.preferences_dialog
                .connect_config_system_theme_sync_notify(move |enabled| {
                    if enabled {
                        app_.setup_system_theme_monitoring().expect("Failed to setup system theme monitoring");
                    }
                    app_.update_theme_from_config().expect("Failed to update theme from config");
                    Inhibit(false)
                });
        }

        {
            let app_ = app.clone();
            app.preferences_dialog
                .connect_config_light_scheme_notify(move |_| {
                    app_.update_theme_from_config().expect("Failed to update theme from config");
                });
        }

        {
            let app_ = app.clone();
            app.preferences_dialog
                .connect_config_dark_scheme_notify(move |_| {
                    app_.update_theme_from_config().expect("Failed to update theme from config");
                });
        }

        // launch config directory in default file manager
        {
            let config_dir_str = XDG_DIRS
                .get_config_home()
                .expect("Failed to get config home")
                .to_string_lossy()
                .to_string();
            let app_ = app.clone();
            app.config_directory_button.connect_clicked(move |_| {
                if let Err(open_err) = open::that(config_dir_str.clone()) {
                    error!("could not launch config directory: {open_err}");
                    app_.post_notification_error(
                        "Failed to launch config directory",
                        NOTIFICATION_LONG_DELAY,
                    );
                }
            });
        }

        // launch more scripts page in default web browser
        {
            let app_ = app.clone();
            app.more_scripts_button.connect_clicked(move |_| {
                if let Err(open_err) = open::that("https://boop.okat.best/scripts/") {
                    error!("could not launch website: {open_err}");
                    app_.post_notification_error(
                        "Failed to launch website",
                        NOTIFICATION_LONG_DELAY,
                    );
                }
            });
        }

        {
            let about_dialog: AboutDialog = app.about_dialog.clone();
            app.about_button.connect_clicked(move |_| {
                let responce = about_dialog.run();
                if responce == gtk::ResponseType::DeleteEvent
                    || responce == gtk::ResponseType::Cancel
                {
                    about_dialog.hide();
                }
            });
        }

        {
            let app_ = app.clone();
            app.shortcuts_button.connect_clicked(move |_| {
                let shortcuts_window = ShortcutsWindow::new();
                shortcuts_window.set_transient_for(Some(&app_.window));
                shortcuts_window.show_all();
            });
        }

        {
            let app_ = app.clone();
            app.header_button.connect_clicked(move |_| {
                app_.run_command_palette()
                    .expect("Failed to run command palette");
            });
        }

        Ok(app)
    }

    fn configure(&self, boop_language: &Language) -> Result<()> {
        self.preferences_dialog
            .set_transient_for(Some(&self.window));

        // update source_view syntax highlighting
        let buffer = self.source_view.get_sourceview_buffer()?;
        buffer.set_highlight_syntax(true);
        buffer.set_language(Some(boop_language));

        Ok(())
    }

    fn update_state_from_config(&self) -> Result<()> {
        let config = self
            .config
            .read()
            .map_err(|e| eyre!("Config lock poisoned: {}", e))?;

        // update source_view style scheme
        let is_dark_mode = self.is_system_dark_mode();
        let scheme_id = config.editor.get_current_scheme_id(is_dark_mode);
        let scheme = sourceview::StyleSchemeManager::get_default()
            .ok_or_else(|| eyre!("Failed to get default style scheme manager"))?
            .get_scheme(scheme_id);
        self.source_view
            .get_sourceview_buffer()?
            .set_style_scheme(scheme.as_ref());

        // update source_view font
        let font_desc = pango::FontDescription::from_string(&config.editor.font_family);
        self.source_view.override_font(&font_desc);

        Ok(())
    }

    fn post_notification(&self, text: &str, delay: u32) {
        let notification_source_id = self.notification_source_id.clone();
        let notification_revealer = self.notification_revealer.clone();
        let notification_label = self.notification_label.clone();

        {
            notification_label.set_markup(text);
            notification_revealer.set_reveal_child(true);

            let mut source_id = notification_source_id.write().unwrap();

            // cancel old notification timeout
            if source_id.is_some() {
                glib::source_remove(source_id.take().unwrap());
            }

            // dismiss after 3000ms
            let new_source_id = {
                let notification_source_id = notification_source_id.clone();
                glib::timeout_add_local(delay, move || {
                    notification_revealer.set_reveal_child(false);
                    *notification_source_id.write().unwrap() = None;
                    Continue(false)
                })
            };

            source_id.replace(new_source_id);
        }
    }

    pub fn post_notification_error(&self, text: &str, delay: u32) {
        self.post_notification(
            &format!(r#"<span foreground="red" weight="bold">ERROR:</span> {text}"#),
            delay,
        );
    }

    pub fn run_command_palette(&self) -> Result<()> {
        let dialog = CommandDialog::new(&self.window, &self.scripts)?;
        dialog.show_all();

        if let gtk::ResponseType::Accept = dialog.run() {
            let selected: &str = dialog
                .get_selected()
                .ok_or_else(|| eyre!("Command palette dialog didn't return a selection"))?;

            *self.last_script_executed.write().unwrap() = Some(String::from(selected));
            self.execute_script(selected)?;
        }

        dialog.close();

        Ok(())
    }

    pub fn re_execute(&self) -> Result<()> {
        if let Some(script_key) = &*self.last_script_executed.read().unwrap() {
            self.execute_script(script_key)
                .wrap_err("Failed to execute script")
        } else {
            warn!("no last script");
            Ok(())
        }
    }

    fn execute_script(&self, script_key: &str) -> Result<()> {
        let mut script_map = self.scripts.write().expect("Scripts lock is poisoned");
        let script: &mut Script = script_map
            .0
            .get_mut(script_key)
            .ok_or_else(|| eyre!("Script not in map"))?;

        info!("executing {}", script.metadata.name);

        let buffer = &self
            .source_view
            .get_buffer()
            .ok_or_else(|| eyre!("Failed to get buffer"))?;

        let buffer_text = buffer
            .get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false)
            .ok_or_else(|| eyre!("Failed to get buffer text"))?;

        let selection_text = buffer
            .get_selection_bounds()
            .and_then(|(start, end)| buffer.get_text(&start, &end, false))
            .map(|s| s.to_string());

        let status_result = script.execute(buffer_text.as_str(), selection_text.as_deref());

        match status_result {
            Ok(status) => {
                // TODO: how to handle multiple messages?
                if let Some(error) = status.error() {
                    self.post_notification(
                        &format!(r#"<span foreground="red" weight="bold">ERROR:</span> {error}"#),
                        NOTIFICATION_LONG_DELAY,
                    );
                } else if let Some(info) = status.info() {
                    self.post_notification(info, NOTIFICATION_LONG_DELAY);
                }
                self.do_replacement(status.clone().into_replacement())
                    .wrap_err_with(|| format!("Failed to make replacement: {status:?}"))?;
            }
            Err(err) => {
                let executor_err = err.downcast::<ExecutorError>().unwrap(); // can't recover from other errors

                error!("Exception: {executor_err:?}");
                self.post_notification_error(
                    &executor_err.into_notification_string(),
                    NOTIFICATION_LONG_DELAY,
                );
            }
        }

        Ok(())
    }

    fn do_replacement(&self, replacement: TextReplacement) -> Result<()> {
        let buffer = &self
            .source_view
            .get_buffer()
            .ok_or_else(|| eyre!("Failed to get buffer"))?;

        match replacement {
            TextReplacement::Full(text) => {
                info!("replacing full text");

                let safe_text = text
                    .remove_null_bytes()
                    .wrap_err("Failed to remove null bytes from text")?;

                buffer.set_text(&safe_text);
            }
            TextReplacement::Selection(text) => {
                info!("replacing selection");

                let safe_text = text
                    .remove_null_bytes()
                    .wrap_err("Failed to remove null bytes from text")?;

                match &mut buffer.get_selection_bounds() {
                    Some((start, end)) => {
                        buffer.delete(start, end);
                        buffer.insert(start, &safe_text);
                    }
                    None => {
                        error!("tried to do a selection replacement, but no text is selected!");
                    }
                }
            }
            TextReplacement::Insert(insertions) => {
                let insert_text = insertions.join("");
                info!("inserting {} bytes", insert_text.len());

                let safe_text = insert_text
                    .remove_null_bytes()
                    .wrap_err("Failed to remove null bytes from text")?;

                if let Some((start, end)) = &mut buffer.get_selection_bounds() {
                    buffer.delete(start, end);
                    buffer.insert(start, &safe_text);
                } else {
                    let mut insert_point =
                        buffer.get_iter_at_offset(buffer.get_property_cursor_position());
                    buffer.insert(&mut insert_point, &safe_text);
                }
            }
            TextReplacement::None => {
                info!("no text to replace");
            }
        }

        self.source_view.grab_focus();

        Ok(())
    }

    fn is_system_dark_mode(&self) -> bool {
        if let Some(settings) = Settings::get_default() {
            if let Some(theme_name) = settings.get_property_gtk_theme_name() {
                // Check if theme name contains "dark" (case insensitive)
                theme_name.to_lowercase().contains("dark")
            } else {
                false
            }
        } else {
            false
        }
    }

    fn setup_system_theme_monitoring(&self) -> Result<()> {
        if let Some(settings) = Settings::get_default() {
            let app = self.clone();
            settings.connect_property_gtk_theme_name_notify(move |_| {
                if let Err(e) = app.update_theme_from_config() {
                    error!("Failed to update theme: {}", e);
                }
            });
        }
        Ok(())
    }

    fn update_theme_from_config(&self) -> Result<()> {
        let config = self
            .config
            .read()
            .map_err(|e| eyre!("Config lock poisoned: {}", e))?;

        if config.editor.sync_with_system_theme {
            let is_dark_mode = self.is_system_dark_mode();
            let scheme_id = config.editor.get_current_scheme_id(is_dark_mode);
            let scheme = sourceview::StyleSchemeManager::get_default()
                .ok_or_else(|| eyre!("Failed to get default style scheme manager"))?
                .get_scheme(scheme_id);
            self.source_view
                .get_sourceview_buffer()?
                .set_style_scheme(scheme.as_ref());
        }

        Ok(())
    }
}
