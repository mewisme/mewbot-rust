use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "mewbot")]
#[command(about = "A production-grade Discord bot built with Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a new plugin (command) template in src/plugins/<name>/
    GeneratePlugin {
        /// Name of the plugin (alphanumeric and underscores)
        name: String,
    },
    /// Show bot version
    Version,
    /// Show config/env value by key
    Config {
        /// Config key to look up
        key: String,
    },
}

pub fn generate_plugin(name: &str) -> Result<(), anyhow::Error> {
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(anyhow::anyhow!(
            "Plugin name must contain only alphanumeric characters and underscores"
        ));
    }

    let plugins_dir = Path::new("src/plugins");
    if !plugins_dir.exists() {
        return Err(anyhow::anyhow!("Plugins directory does not exist"));
    }

    let plugin_dir = plugins_dir.join(name);
    if plugin_dir.exists() {
        return Err(anyhow::anyhow!(
            "Plugin directory already exists: {}",
            plugin_dir.display()
        ));
    }

    fs::create_dir_all(&plugin_dir)?;

    let struct_name = to_pascal_case(name);

    let mod_content = format!(
        r#"mod command;

pub use command::{{create, {0}}};
"#,
        struct_name
    );
    fs::write(plugin_dir.join("mod.rs"), mod_content)?;
    crate::done!("Created src/plugins/{}/mod.rs", name);

    let command_template = format!(
        r#"use crate::core::command::SubCommandInfo;
use crate::core::Command;
use async_trait::async_trait;
use serenity::builder::{{
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
}};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;

pub struct {0};

#[async_trait]
impl Command for {0} {{
    fn name(&self) -> &'static str {{
        "{1}"
    }}

    fn description(&self) -> &'static str {{
        "Description of {1} command"
    }}

    fn register_slash(&self, cmd: &mut CreateCommand) {{
        *cmd = CreateCommand::new("{1}")
            .description("Description of {1} command");
    }}

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {{
        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Response from {1} command"),
                ),
            )
            .await?;
        Ok(())
    }}

    fn prefix(&self) -> Option<&'static str> {{
        Some("{1}")
    }}

    async fn run_prefix(
        &self,
        ctx: &Context,
        msg: &Message,
        _args: &[&str],
    ) -> anyhow::Result<()> {{
        msg.channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().content("Response from {1} command"),
            )
            .await?;
        Ok(())
    }}

    fn version(&self) -> &'static str {{
        "1.0.0"
    }}

    fn subcommands(&self) -> &'static [SubCommandInfo] {{
        // SubCommandInfo {{ name: "sub", description: "...", aliases: &[] }}
        &[]
    }}
}}

pub fn create() -> Arc<dyn Command> {{
    Arc::new({0})
}}
"#,
        struct_name,
        name,
    );
    fs::write(plugin_dir.join("command.rs"), command_template)?;
    crate::done!("Created src/plugins/{}/command.rs", name);

    let plugins_mod_path = Path::new("src/plugins/mod.rs");
    let mod_line = format!("mod {};", name);

    if plugins_mod_path.exists() {
        let content = fs::read_to_string(plugins_mod_path)?;
        let mut new_content = content.clone();

        if !content.contains(&mod_line) {
            if let Some(pos) = content.find("mod wallet;") {
                let end = pos + "mod wallet;".len();
                new_content.insert_str(end, &format!("\nmod {};", name));
            } else if let Some(pos) = content.find('\n') {
                new_content.insert_str(pos + 1, &format!("mod {};\n", name));
            } else {
                new_content = format!("{}\n{}", mod_line, new_content);
            }
        }

        if !content.contains(&format!("{}::create()", name)) {
            let needle = "    let help_cmd = help::create(api)";
            if let Some(pos) = new_content.find(needle) {
                let block = format!(
                    "    let {}_cmd = {}::create();\n    registry.register({}_cmd.clone());\n    crate::done!(\"Loaded plugin {{}} v{{}}\", {}_cmd.name(), {}_cmd.version());\n\n    ",
                    name, name, name, name, name
                );
                new_content.insert_str(pos, &block);
            }
        }

        fs::write(plugins_mod_path, new_content)?;
        crate::done!("Updated src/plugins/mod.rs");
    }

    crate::info!("\nNext steps:");
    crate::info!("1. Edit src/plugins/{}/command.rs to implement your logic", name);
    crate::info!("2. Rebuild and run the bot");

    Ok(())
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

pub fn show_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    crate::info!("Mewbot v{}", VERSION);
}

pub fn show_config(key: &str) -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    match std::env::var(key) {
        Ok(value) => {
            crate::info!("{}={}", key, value);
        }
        Err(std::env::VarError::NotPresent) => {
            return Err(anyhow::anyhow!(
                "Config key '{}' not found in environment",
                key
            ));
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Error reading config key '{}': {}", key, e));
        }
    }

    Ok(())
}
