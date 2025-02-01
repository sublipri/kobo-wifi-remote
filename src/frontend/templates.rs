use askama::Template;
use chrono::Duration;

use super::index::IndexItem;
use crate::{
    actions::ActionDetails,
    config::{CustomActionOptions, PageTurnerOptions, RemoteOptions, SetupOptions},
    frontend::index::IndexOptions,
    kobo_config::KoboConfigSetting,
};

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub opts: &'a IndexOptions,
    pub items: Vec<IndexItem<'a>>,
}

#[derive(Template)]
#[template(path = "setup.html")]
pub struct Setup {
    pub opts: SetupOptions,
    pub is_sunxi: bool,
    pub device_name: String,
    pub fbink_is_err: bool,
    pub fbink_is_disabled: bool,
}

#[derive(Template)]
#[template(path = "custom-actions.html")]
pub struct CustomActions {
    pub opts: CustomActionOptions,
}

#[derive(Template)]
#[template(path = "page-turner.html")]
pub struct PageTurner {
    pub next: Option<ActionDetails>,
    pub prev: Option<ActionDetails>,
    pub opts: PageTurnerOptions,
}

#[derive(Template)]
#[template(path = "troubleshooting.html")]
pub struct Troubleshooting {}

#[derive(Template)]
#[template(path = "developer-settings.html")]
pub struct DeveloperSettings {
    pub settings: Vec<KoboConfigSetting>,
}

#[derive(Template)]
#[template(path = "remote-control.html")]
pub struct RemoteControl {
    pub actions: Vec<ActionDetails>,
    pub shortcuts_json: String,
    pub opts: RemoteOptions,
}

#[derive(Template)]
#[template(path = "manage-actions.html")]
pub struct ManageActions {
    pub actions: Vec<ActionDetails>,
}

#[derive(Template)]
#[template(path = "auto-turner.html")]
pub struct AutoTurner {
    pub next: Option<ActionDetails>,
    pub prev: Option<ActionDetails>,
    pub delay: Duration,
}

#[derive(Template)]
#[template(path = "voice-activation.html")]
pub struct VoiceActivation {
    pub language_code: String,
}

#[derive(Template)]
#[template(path = "edit-config.html")]
pub struct EditConfig {
    pub config: String,
}
