use askama::Template;

use crate::actions::Action;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {}

#[derive(Template)]
#[template(path = "setup.html")]
pub struct Setup {}

#[derive(Template)]
#[template(path = "custom-actions.html")]
pub struct CustomActions {}

#[derive(Template)]
#[template(path = "page-turner.html")]
pub struct PageTurner {}

#[derive(Template)]
#[template(path = "troubleshooting.html")]
pub struct Troubleshooting {}

#[derive(Template)]
#[template(path = "remote-control.html")]
pub struct RemoteControl {
    pub actions: Vec<Action>,
    pub shortcuts_json: String,
}

#[derive(Template)]
#[template(path = "manage-actions.html")]
pub struct ManageActions {
    pub actions: Vec<Action>,
}
