use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexOptions {
    pub button_height: f32,
    pub items: Vec<IndexItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexItem {
    pub href: String,
    pub name: String,
    pub enabled: bool,
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            button_height: 20.0,
            items: vec![
                IndexItem::new("setup", "Initial Setup", true),
                IndexItem::new("page-turner", "Page Turner", true),
                IndexItem::new("custom-actions", "Custom Actions", true),
                IndexItem::new("remote-control", "Remote Control", true),
                IndexItem::new("screenshot", "Screenshot", true),
                IndexItem::new("auto-turner", "Auto Turner", true),
                IndexItem::new("developer-settings", "Developer Settings", true),
                IndexItem::new("troubleshooting", "Troubleshooting", true),
            ],
        }
    }
}

impl IndexItem {
    pub fn new(href: &str, name: &str, enabled: bool) -> Self {
        Self {
            href: href.to_string(),
            name: name.to_string(),
            enabled,
        }
    }
}
