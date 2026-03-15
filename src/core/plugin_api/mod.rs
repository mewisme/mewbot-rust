use crate::core::command::CommandLister;
use crate::core::config::Config;
use std::sync::Arc;

#[derive(Clone)]
pub struct PluginApi {
    pub config: Config,
    pub command_lister: Arc<dyn CommandLister>,
}

impl PluginApi {
    pub fn new(config: Config, command_lister: Arc<dyn CommandLister>) -> Self {
        Self {
            config,
            command_lister,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn command_lister(&self) -> &Arc<dyn CommandLister> {
        &self.command_lister
    }
}
