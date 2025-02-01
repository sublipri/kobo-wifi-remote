use crate::config::Config;

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use fbink_rs::{config::Font, FbInk, FbInkConfig};
use tracing::error;

#[derive(Clone, Debug)]
pub enum FbInkWrapper {
    Loaded(Arc<FbInk>),
    Failed(String), // anyhow::Result is !Clone so we just store the error as a string
    DisabledInConfig,
    DisabledAtBuild, // TODO
}

impl FbInkWrapper {
    pub fn new(config: &Config) -> Self {
        if !config.user.fbink.enabled {
            return Self::DisabledInConfig;
        }

        let fbink = FbInk::new(FbInkConfig {
            is_centered: true,
            is_halfway: true,
            is_padded: true,
            font: Font::Fatty,
            to_syslog: true,
            ..Default::default()
        })
        .context("Failed to initialize FBInk");

        match fbink {
            Ok(fbink) => Self::Loaded(Arc::new(fbink)),
            Err(e) => {
                error!("{e}");
                Self::Failed(e.to_string())
            }
        }
    }

    pub fn try_inner(&self) -> Result<&Arc<FbInk>> {
        match self {
            FbInkWrapper::Loaded(i) => Ok(i),
            FbInkWrapper::Failed(e) => bail!(e.clone()),
            FbInkWrapper::DisabledInConfig => bail!("FBInk is disabled in the user config"),
            FbInkWrapper::DisabledAtBuild => {
                bail!("This version of the remote was built without FBInk support")
            }
        }
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::DisabledInConfig | Self::DisabledAtBuild)
    }
}
