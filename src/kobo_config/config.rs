use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

use ini_roundtrip::{Item, Parser};
use tracing::{debug, warn};

/// Options to configure how we handle the config file
pub struct KoboConfigOptions {
    // Whitelist the keys allowed to be edited to minimize the risk of breaking things
    // and stop someone on the same network from sneakily enabling a root telnet server
    pub whitelist: BTreeMap<String, Vec<String>>,
    pub validate_types: bool,
    pub path: PathBuf,
}

impl Default for KoboConfigOptions {
    fn default() -> Self {
        Self {
            path: "/mnt/onboard/.kobo/Kobo/Kobo eReader.conf".into(),
            validate_types: true,
            whitelist: BTreeMap::from([(
                "DeveloperSettings".into(),
                vec![
                    "ForceWifiOn".into(),
                    "ForceAllowLandscape".into(),
                    "AutoUsbGadget".into(),
                    "ShowKeyboardTaps".into(),
                ],
            )]),
        }
    }
}

/// A single setting that might be present in the INI file
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct KoboConfigSetting {
    pub section: String,
    pub key: String,
    // Empty string = key is present in config with empty value. None = key not present in config.
    pub value: Option<String>,
}

/// The 'Kobo eReader.conf' Qsettings INI file
pub struct KoboConfigFile {
    pub contents: String,
    pub opts: KoboConfigOptions,
}

impl KoboConfigFile {
    pub fn open(opts: KoboConfigOptions) -> Result<Self> {
        Ok(Self {
            contents: fs::read_to_string(&opts.path)?,
            opts,
        })
    }

    /// Write the contents to disk
    pub fn write(&self) -> Result<()> {
        fs::copy(
            &self.opts.path,
            self.opts.path.with_extension("conf.wrfbkp"),
        )?;
        let tmp = self.opts.path.with_extension("conf.wfrtmp");
        fs::write(&tmp, &self.contents)?;
        fs::rename(&tmp, &self.opts.path)?;
        Ok(())
    }

    /// Get the values for all the whitelisted settings
    pub fn get_values(&self) -> Vec<KoboConfigSetting> {
        let mut current_section = "";
        let mut settings = Vec::new();
        // Get whitelisted settings present in config file
        for item in Parser::new(&self.contents) {
            match item {
                Item::Section { name, .. } => {
                    current_section = name;
                }
                Item::Property { key, val, .. } => {
                    if self.is_whitelisted(current_section, key) {
                        let setting = KoboConfigSetting {
                            section: current_section.to_string(),
                            key: key.to_string(),
                            value: Some(val.unwrap_or_default().to_string()),
                        };
                        settings.push(setting)
                    }
                }
                Item::SectionEnd => {
                    current_section = "";
                }
                _ => continue,
            }
        }
        // Get whitelisted settings not present in config file
        for (section, keys) in self.opts.whitelist.iter() {
            for key in keys {
                if !settings
                    .iter()
                    .any(|s| &s.section == section && &s.key == key)
                {
                    settings.push(KoboConfigSetting {
                        section: section.to_string(),
                        key: key.to_string(),
                        value: None,
                    })
                }
            }
        }
        settings
    }

    fn is_whitelisted(&self, section: &str, key: &str) -> bool {
        let Some(keys) = self.opts.whitelist.get(section) else {
            return false;
        };
        keys.iter().any(|k| k == key)
    }

    // This isn't very efficient for setting multiple values but should suffice for our needs
    // Not using a higher-level INI library just in case Kobo doesn't like the way they alter the
    // capitalization or formatting. Tried one that couldn't handle their QSettings @Variant
    // values so thought it better to be safe and only alter the lines we want to change.
    /// Change a setting in the given section to the given value
    pub fn set_value(
        &mut self,
        section: &str,
        key_to_set: &str,
        new_val: Option<&str>,
    ) -> Result<()> {
        if !self.is_whitelisted(section, key_to_set) {
            return Err(anyhow!("Changing {key_to_set} isn't allowed"));
        }

        let mut current_section = None;
        let mut was_changed = false;
        let mut updated = String::with_capacity(self.contents.capacity());
        for (index, item) in Parser::new(&self.contents).enumerate() {
            match item {
                Item::Error(line) => {
                    warn!(
                        "Malformed header in {} at line {}: {line}",
                        self.opts.path.display(),
                        index + 1,
                    );
                    updated.push_str(line);
                    updated.push('\n');
                }
                Item::Section { name, raw } => {
                    current_section = Some(name);
                    updated.push_str(raw);
                    updated.push('\n');
                }
                Item::Property { key, val, raw } => {
                    if Some(section) == current_section && key == key_to_set {
                        let Some(new_val) = new_val else { continue };
                        if self.opts.validate_types {
                            self.validate_types(val.unwrap_or_default(), new_val)?;
                        }
                        updated.push_str(&format!("{key}={new_val}\n"));
                        let (old_val, path) = (val.unwrap_or_default(), self.opts.path.display());
                        debug!("Changing {key} in {path} from {old_val} to {new_val}",);
                        was_changed = true;
                    } else {
                        updated.push_str(raw);
                        updated.push('\n')
                    }
                }
                Item::SectionEnd => {
                    if Some(section) == current_section && !was_changed {
                        let Some(new_val) = new_val else { continue };
                        if updated.ends_with("\n\n") {
                            updated.pop();
                            updated.push_str(&format!("{key_to_set}={new_val}\n\n"));
                        } else {
                            updated.push_str(&format!("{key_to_set}={new_val}\n"));
                        }
                        was_changed = true;
                    }
                    current_section = None;
                }
                Item::Comment { raw } => {
                    updated.push_str(raw);
                    updated.push('\n');
                }
                Item::Blank { raw } => {
                    updated.push_str(raw);
                    updated.push('\n');
                }
            }
        }

        if !was_changed {
            if let Some(new_val) = new_val {
                updated.push_str(&format!("\n[{section}]\n"));
                updated.push_str(&format!("{key_to_set}={new_val}\n"))
            }
        }
        self.contents = updated;
        Ok(())
    }

    /// Ensure the new value is the same type as the old value
    fn validate_types(&self, old_val: &str, new_val: &str) -> Result<()> {
        if old_val.to_lowercase().parse::<bool>().is_ok()
            && new_val.to_lowercase().parse::<bool>().is_err()
        {
            Err(anyhow!("Can't replace boolean with non-boolean value"))
        } else if old_val.parse::<i64>().is_ok() && new_val.parse::<i64>().is_err() {
            Err(anyhow!("Can't replace integer with non-integer value"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KoboConfigFile, KoboConfigOptions, KoboConfigSetting};

    use indoc::indoc;

    #[test]
    fn set_value_override_existing() {
        let cfg = indoc! {"
            [DeveloperSettings]
            EnableDebugServices=true
            ForceWifiOn=false
            ForceAllowLandscape=true
            ShowKeyboardTaps=false
        "};
        let expected = indoc! {"
            [DeveloperSettings]
            EnableDebugServices=true
            ForceWifiOn=true
            ForceAllowLandscape=true
            ShowKeyboardTaps=false
        "};

        let mut config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };
        config
            .set_value("DeveloperSettings", "ForceWifiOn", Some("true"))
            .unwrap();
        assert_eq!(&config.contents, expected);
    }

    #[test]
    fn set_value_not_present() {
        let cfg = indoc! {"
            [DeveloperSettings]
            EnableDebugServices=true
            ForceAllowLandscape=true
            ShowKeyboardTaps=false

            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true
        "};
        let expected = indoc! {"
            [DeveloperSettings]
            EnableDebugServices=true
            ForceAllowLandscape=true
            ShowKeyboardTaps=false
            ForceWifiOn=true

            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true
        "};

        let mut config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };
        config
            .set_value("DeveloperSettings", "ForceWifiOn", Some("true"))
            .unwrap();
        assert_eq!(&config.contents, expected);
    }

    #[test]
    fn set_value_no_section() {
        let cfg = indoc! {"
            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true
        "};
        let expected = indoc! {"
            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true

            [DeveloperSettings]
            ForceWifiOn=false
        "};

        let mut config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };
        config
            .set_value("DeveloperSettings", "ForceWifiOn", Some("false"))
            .unwrap();
        assert_eq!(&config.contents, expected);
    }

    #[test]
    fn set_invalid_boolean_fails() {
        let cfg = indoc! {"
            [DeveloperSettings]
            ForceWifiOn=false
        "};
        let mut config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };
        let result = config.set_value("DeveloperSettings", "ForceWifiOn", Some("indeed"));
        assert!(result.is_err());
    }

    #[test]
    fn set_not_whitelisted_fails() {
        let mut config = KoboConfigFile {
            contents: String::new(),
            opts: KoboConfigOptions::default(),
        };
        let result = config.set_value("DeveloperSettings", "EnableDebugServices", Some("true"));
        assert!(result.is_err());
    }

    #[test]
    fn get_values_is_set() {
        let cfg = indoc! {"
            [DeveloperSettings]
            AutoUsbGadget=true
            ForceAllowLandscape=true
            ShowKeyboardTaps=false
            ForceWifiOn=false
            EnableDebugServices=false

            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true
        "};
        let config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };

        let values = config.get_values();
        let expected = vec![
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "AutoUsbGadget".into(),
                value: Some("true".into()),
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ForceAllowLandscape".into(),
                value: Some("true".into()),
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ShowKeyboardTaps".into(),
                value: Some("false".into()),
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ForceWifiOn".into(),
                value: Some("false".into()),
            },
        ];
        assert_eq!(values, expected);
    }
    #[test]
    fn get_values_not_set() {
        let cfg = indoc! {"
            [DialogSettings]
            ReleaseNotesShown=true
            ReturningReaderDialogShown=true
        "};
        let config = KoboConfigFile {
            contents: cfg.to_string(),
            opts: KoboConfigOptions::default(),
        };

        let values = config.get_values();
        let expected = vec![
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ForceWifiOn".into(),
                value: None,
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ForceAllowLandscape".into(),
                value: None,
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "AutoUsbGadget".into(),
                value: None,
            },
            KoboConfigSetting {
                section: "DeveloperSettings".into(),
                key: "ShowKeyboardTaps".into(),
                value: None,
            },
        ];
        assert_eq!(values, expected);
    }
}
