use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexItem<'a> {
    pub href: &'static str,
    pub name: &'a str,
    pub sort_value: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexItemOptions {
    pub visible: bool,
    pub display_name: String,
    pub sort_value: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexOptions {
    pub button_height: f32,
    pub color1: String,
    pub color2: String,
    pub setup: IndexItemOptions,
    pub page_turner: IndexItemOptions,
    pub custom_actions: IndexItemOptions,
    pub remote_control: IndexItemOptions,
    pub screenshot: IndexItemOptions,
    pub edit_config: IndexItemOptions,
    pub auto_turner: IndexItemOptions,
    pub voice_activation: IndexItemOptions,
    pub developer_settings: IndexItemOptions,
    pub troubleshooting: IndexItemOptions,
}

fn opts(display_name: &str, sort_value: u8, visible: bool) -> IndexItemOptions {
    IndexItemOptions {
        visible,
        display_name: display_name.into(),
        sort_value,
    }
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            button_height: 20.0,
            color1: "#33b249".into(),
            color2: "#5783db".into(),
            setup: opts("Initial Setup", 10, true),
            page_turner: opts("Page Turner", 20, true),
            custom_actions: opts("Custom Actions", 30, true),
            remote_control: opts("Remote Control", 40, true),
            screenshot: opts("Screenshot", 50, true),
            edit_config: opts("Edit Config", 60, true),
            auto_turner: opts("Auto Turner", 70, true),
            voice_activation: opts("Voice Activation", 80, true),
            developer_settings: opts("Developer Settings", 90, true),
            troubleshooting: opts("Troubleshooting", 100, true),
        }
    }
}

impl IndexOptions {
    pub fn items(&self) -> Vec<IndexItem> {
        let mut items = Vec::with_capacity(10);
        for (href, opts) in [
            ("setup", &self.setup),
            ("page-turner", &self.page_turner),
            ("custom-actions", &self.custom_actions),
            ("remote-control", &self.remote_control),
            ("edit-config", &self.edit_config),
            ("screenshot", &self.screenshot),
            ("auto-turner", &self.auto_turner),
            ("voice-activation", &self.voice_activation),
            ("developer-settings", &self.developer_settings),
            ("troubleshooting", &self.troubleshooting),
        ] {
            if opts.visible {
                items.push(IndexItem {
                    href,
                    name: &opts.display_name,
                    sort_value: opts.sort_value,
                })
            }
        }
        items.sort_by_key(|i| i.sort_value);
        items
    }
}
