use askama::Template;

use super::index::IndexItem;
use crate::{actions::ListActionResponse, kobo_config::KoboConfigSetting};

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub button_height: f32,
    pub items: Vec<&'a IndexItem>,
}

#[derive(Template)]
#[template(path = "setup.html")]
pub struct Setup {}

#[derive(Template)]
#[template(path = "custom-actions.html")]
pub struct CustomActions {}

#[derive(Template)]
#[template(path = "page-turner.html")]
pub struct PageTurner {
    pub next: Option<ListActionResponse>,
    pub prev: Option<ListActionResponse>,
    pub enable_arbitrary: bool,
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
    pub actions: Vec<ListActionResponse>,
    pub shortcuts_json: String,
    pub enable_arbitrary: bool,
}

#[derive(Template)]
#[template(path = "manage-actions.html")]
pub struct ManageActions {
    pub actions: Vec<ListActionResponse>,
}
