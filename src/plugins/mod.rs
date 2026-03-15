mod help;
mod wallet;

use crate::core::plugin_api::PluginApi;
use crate::core::registry::Registry;
use crate::core::Command;

pub fn register_commands(registry: &mut Registry, api: PluginApi) {
    let wallet_cmd = wallet::create();
    registry.register(wallet_cmd.clone());
    crate::done!(
        "Loaded plugin {} v{} 📦",
        wallet_cmd.name(),
        wallet_cmd.version()
    );

    let help_cmd = help::create(api);
    registry.register(help_cmd.clone());
    crate::done!(
        "Loaded plugin {} v{} 📦",
        help_cmd.name(),
        help_cmd.version()
    );
}
