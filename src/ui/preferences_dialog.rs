use std::sync::{Arc, RwLock};

use eyre::{Context, Result};
use gladis::Gladis;
use glib::SignalHandlerId;
use gtk::{prelude::*, Dialog as GtkDialog, FontButton, Switch};
use pango::prelude::*;
use sourceview::{StyleScheme, StyleSchemeChooserExt, StyleSchemeExt, StyleSchemeManagerExt};

use crate::config::Config;

#[derive(Gladis, Clone, Shrinkwrap)]
pub struct Widgets {
    #[shrinkwrap(main_field)]
    preference_dialog: GtkDialog, // TODO: change to preferences_dialog

    color_scheme_button: sourceview::StyleSchemeChooserButton,
    shortcut_switch: Switch,
    font_button: FontButton,
    monospace_filter_switch: Switch,
    
    // System theme sync widgets
    system_theme_sync_switch: Switch,
    light_scheme_button: sourceview::StyleSchemeChooserButton,
    dark_scheme_button: sourceview::StyleSchemeChooserButton,
}

#[derive(Clone, Shrinkwrap)]
pub struct Dialog {
    #[shrinkwrap(main_field)]
    widgets: Widgets,
    config: Arc<RwLock<Config>>,
}

impl Dialog {
    pub(crate) fn new(config: &Arc<RwLock<Config>>) -> Result<Self> {
        let mut dialog = Dialog {
            widgets: Widgets::from_resource("/fyi/zoey/Boop-GTK/boop-gtk.glade")
                .wrap_err("Failed to load boop-gtk.glade")?,
            config: config.clone(),
        };

        dialog.update_state_from_config()?;
        dialog.connect_config_style_scheme_notify(Dialog::on_config_style_scheme_notify(
            config.clone(),
        ));
        dialog.connect_config_open_shortcuts_on_startup_notify(
            Dialog::on_config_open_shortcuts_on_startup_notify(config.clone()),
        );
        dialog.connect_config_font_notify(Dialog::on_config_font_notify(config.clone()));
        dialog.connect_config_monospace_filter_notify(Dialog::on_config_monospace_filter_notify(config.clone()));
        dialog.connect_config_system_theme_sync_notify(Dialog::on_config_system_theme_sync_notify(config.clone()));
        dialog.connect_config_light_scheme_notify(Dialog::on_config_light_scheme_notify(config.clone()));
        dialog.connect_config_dark_scheme_notify(Dialog::on_config_dark_scheme_notify(config.clone()));
        
        // Update font filter when monospace filter switch changes
        {
            let dialog_clone = dialog.clone();
            dialog.monospace_filter_switch.connect_state_set(move |_, _| {
                dialog_clone.update_font_filter();
                Inhibit(false)
            });
        }
        
        // Update theme chooser visibility when system sync switch changes
        {
            let dialog_clone = dialog.clone();
            dialog.system_theme_sync_switch.connect_state_set(move |_, _| {
                dialog_clone.update_theme_chooser_visibility();
                Inhibit(false)
            });
        }
        
        // Initialize font filter and theme chooser visibility based on config
        dialog.update_font_filter();
        dialog.update_theme_chooser_visibility();

        Ok(dialog)
    }

    // update the controls with values from config
    pub fn update_state_from_config(&mut self) -> Result<()> {
        let config = self
            .config
            .read()
            .map_err(|e| eyre!("Config lock poisoned: {}", e))?;

        // update color_scheme_button
        let scheme_id = &config.editor.colour_scheme_id;
        let scheme = sourceview::StyleSchemeManager::get_default()
            .ok_or_else(|| eyre!("Failed to get default style scheme manager"))?
            .get_scheme(scheme_id)
            .ok_or_else(|| eyre!("StyleSchemeManager could not find scheme '{}'", scheme_id))?;
        self.color_scheme_button.set_style_scheme(&scheme);

        // update shortcut_switch
        self.shortcut_switch
            .set_state(config.show_shortcuts_on_open);

        // update font_button
        self.font_button.set_font(&config.editor.font_family);

        // update monospace_filter_switch
        self.monospace_filter_switch
            .set_state(config.editor.monospace_filter);

        // update system_theme_sync_switch
        self.system_theme_sync_switch
            .set_state(config.editor.sync_with_system_theme);

        // update light_scheme_button
        let light_scheme = sourceview::StyleSchemeManager::get_default()
            .ok_or_else(|| eyre!("Failed to get default style scheme manager"))?
            .get_scheme(&config.editor.light_scheme_id)
            .ok_or_else(|| eyre!("StyleSchemeManager could not find light scheme '{}'", config.editor.light_scheme_id))?;
        self.light_scheme_button.set_style_scheme(&light_scheme);

        // update dark_scheme_button
        let dark_scheme = sourceview::StyleSchemeManager::get_default()
            .ok_or_else(|| eyre!("Failed to get default style scheme manager"))?
            .get_scheme(&config.editor.dark_scheme_id)
            .ok_or_else(|| eyre!("StyleSchemeManager could not find dark scheme '{}'", config.editor.dark_scheme_id))?;
        self.dark_scheme_button.set_style_scheme(&dark_scheme);

        Ok(())
    }

    fn on_config_style_scheme_notify(config: Arc<RwLock<Config>>) -> impl Fn(Option<StyleScheme>) {
        move |scheme: Option<StyleScheme>| {
            if let Some(scheme_id) = scheme.and_then(|s| s.get_id()) {
                let mut config = config.write().expect("Config lock poisoned");
                config.editor.set_colour_scheme_id(scheme_id.as_str());
                config.save().expect("Failed to save config");
            } else {
                error!("Style scheme is None");
            }
        }
    }

    fn on_config_open_shortcuts_on_startup_notify(
        config: Arc<RwLock<Config>>,
    ) -> impl Fn(bool) -> Inhibit {
        move |enabled| {
            let mut config = config.write().expect("Config lock poisoned");
            config.set_show_shortcuts_on_open(enabled);
            config.save().expect("Failed to save config");

            Inhibit(false)
        }
    }

    fn on_config_font_notify(config: Arc<RwLock<Config>>) -> impl Fn(&str) {
        move |font| {
            let mut config = config.write().expect("Config lock poisoned");
            config.editor.set_font_family(font);
            config.save().expect("Failed to save config");
        }
    }

    fn on_config_monospace_filter_notify(config: Arc<RwLock<Config>>) -> impl Fn(bool) -> Inhibit {
        move |enabled| {
            let mut config = config.write().expect("Config lock poisoned");
            config.editor.set_monospace_filter(enabled);
            config.save().expect("Failed to save config");

            Inhibit(false)
        }
    }

    fn on_config_system_theme_sync_notify(config: Arc<RwLock<Config>>) -> impl Fn(bool) -> Inhibit {
        move |enabled| {
            let mut config = config.write().expect("Config lock poisoned");
            config.editor.set_sync_with_system_theme(enabled);
            config.save().expect("Failed to save config");

            Inhibit(false)
        }
    }

    fn on_config_light_scheme_notify(config: Arc<RwLock<Config>>) -> impl Fn(Option<StyleScheme>) {
        move |scheme: Option<StyleScheme>| {
            if let Some(scheme_id) = scheme.and_then(|s| s.get_id()) {
                let mut config = config.write().expect("Config lock poisoned");
                config.editor.set_light_scheme_id(scheme_id.as_str());
                config.save().expect("Failed to save config");
            } else {
                error!("Light style scheme is None");
            }
        }
    }

    fn on_config_dark_scheme_notify(config: Arc<RwLock<Config>>) -> impl Fn(Option<StyleScheme>) {
        move |scheme: Option<StyleScheme>| {
            if let Some(scheme_id) = scheme.and_then(|s| s.get_id()) {
                let mut config = config.write().expect("Config lock poisoned");
                config.editor.set_dark_scheme_id(scheme_id.as_str());
                config.save().expect("Failed to save config");
            } else {
                error!("Dark style scheme is None");
            }
        }
    }

    pub fn connect_config_style_scheme_notify<F: Fn(Option<StyleScheme>) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.color_scheme_button
            .connect_property_style_scheme_notify(move |button| f(button.get_style_scheme()))
    }

    pub fn connect_config_open_shortcuts_on_startup_notify<F: Fn(bool) -> Inhibit + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.shortcut_switch
            .connect_state_set(move |_, state| f(state))
    }

    pub fn connect_config_font_notify<F: Fn(&str) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.font_button
            .connect_font_set(move |button| {
                if let Some(font_name) = button.get_font() {
                    f(&font_name);
                }
            })
    }

    pub fn connect_config_monospace_filter_notify<F: Fn(bool) -> Inhibit + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.monospace_filter_switch
            .connect_state_set(move |_, state| f(state))
    }

    pub fn connect_config_system_theme_sync_notify<F: Fn(bool) -> Inhibit + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.system_theme_sync_switch
            .connect_state_set(move |_, state| f(state))
    }

    pub fn connect_config_light_scheme_notify<F: Fn(Option<StyleScheme>) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.light_scheme_button
            .connect_property_style_scheme_notify(move |button| f(button.get_style_scheme()))
    }

    pub fn connect_config_dark_scheme_notify<F: Fn(Option<StyleScheme>) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.dark_scheme_button
            .connect_property_style_scheme_notify(move |button| f(button.get_style_scheme()))
    }

    fn update_font_filter(&self) {
        let config = self.config.read().expect("Config lock poisoned");
        let filter_enabled = config.editor.monospace_filter;
        
        if filter_enabled {
            self.font_button.set_filter_func(Some(Box::new(|family, _face| {
                family.is_monospace()
            })));
        } else {
            self.font_button.set_filter_func(None);
        }
    }

    fn update_theme_chooser_visibility(&self) {
        let config = self.config.read().expect("Config lock poisoned");
        let sync_enabled = config.editor.sync_with_system_theme;
        
        // When sync is enabled, show light/dark choosers and hide main chooser
        // When sync is disabled, show main chooser and hide light/dark choosers
        self.color_scheme_button.set_visible(!sync_enabled);
        self.light_scheme_button.set_visible(sync_enabled);
        self.dark_scheme_button.set_visible(sync_enabled);
    }
}
