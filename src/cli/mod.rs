use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "discord-bot-rust")]
#[command(about = "A production-grade Discord bot built with Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a base command template file
    Generate {
        /// Name of the command to generate
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

pub fn generate_command(name: &str) -> Result<(), anyhow::Error> {
    // Validate command name (alphanumeric and underscores only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(anyhow::anyhow!(
            "Command name must contain only alphanumeric characters and underscores"
        ));
    }

    let commands_dir = Path::new("src/commands");
    if !commands_dir.exists() {
        return Err(anyhow::anyhow!("Commands directory does not exist"));
    }

    let file_path = commands_dir.join(format!("{}.rs", name));
    if file_path.exists() {
        return Err(anyhow::anyhow!(
            "Command file already exists: {}",
            file_path.display()
        ));
    }

    let template = format!(
        r#"use crate::commands::Command;
use async_trait::async_trait;
use serenity::builder::{{
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    CreateEmbed
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
        interaction.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Response from {1} command")
            )
        ).await?;
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

        let builder = CreateMessage::new()
            .content("Response from {1} command");

        msg.channel_id.send_message(&ctx.http, builder).await?;

        Ok(())
    }}

    fn aliases(&self) -> &'static [&'static str] {{
        &[]
    }}

    fn cooldown_duration(&self) -> Duration {{
        Duration::from_secs(3)
    }}
}}

pub fn create() -> Arc<dyn Command> {{
    Arc::new({0})
}}
      "#,
        capitalize_first(name),
        name,
    );

    fs::write(&file_path, template)?;
    crate::done!("Generated command file: {}", file_path.display());

    // Automatically add mod declaration to commands/mod.rs
    let mod_rs_path = Path::new("src/commands/mod.rs");
    if mod_rs_path.exists() {
        let mod_rs_content = fs::read_to_string(mod_rs_path)?;
        let mod_line = format!("pub mod {};", name);

        // Check if module is already declared
        if !mod_rs_content.contains(&mod_line) {
            let lines: Vec<&str> = mod_rs_content.lines().collect();
            let mut new_lines = Vec::new();
            let mut inserted = false;

            // Find the last mod declaration and insert after it
            for (idx, line) in lines.iter().enumerate() {
                new_lines.push(*line);

                // Check if this is a mod declaration
                if line.trim().starts_with("pub mod ") {
                    // Check if this is the last mod declaration
                    let is_last_mod = lines[idx + 1..]
                        .iter()
                        .all(|l| !l.trim().starts_with("pub mod "));

                    if is_last_mod && !inserted {
                        // Insert the new mod declaration after this one
                        new_lines.push(&mod_line);
                        inserted = true;
                    }
                }
            }

            // If we didn't insert (no mod declarations found), add at the end
            if !inserted {
                new_lines.push(&mod_line);
            }

            let updated_content = new_lines.join("\n");
            fs::write(mod_rs_path, updated_content)?;
            crate::done!("✓ Added `{}` to src/commands/mod.rs", mod_line);
        } else {
            crate::info!("✓ Module already declared in src/commands/mod.rs");
        }
    }

    // Automatically add registration to utils::register_commands
    let utils_rs_path = Path::new("src/utils/mod.rs");
    if utils_rs_path.exists() {
        let utils_content = fs::read_to_string(utils_rs_path)?;
        let reg_line = format!("    registry.register({}::create());", name);

        // Check if registration is already present
        if !utils_content.contains(&reg_line) {
            let lines: Vec<&str> = utils_content.lines().collect();
            let mut new_lines = Vec::new();
            let mut inserted = false;

            // Find the register_commands function and insert after the last registry.register() call
            // but before the help command registration
            for (idx, line) in lines.iter().enumerate() {
                new_lines.push(*line);

                // Look for registry.register() calls
                if line.trim().starts_with("registry.register(") && !line.contains("help::create") {
                    // Check if this is the last regular registration (before help command)
                    let is_last_reg = lines[idx + 1..].iter().any(|l| {
                        l.contains("help::create") || l.trim().starts_with("// Help command")
                    });

                    if is_last_reg && !inserted {
                        // Insert the new registration after this one
                        new_lines.push(&reg_line);
                        inserted = true;
                    }
                }
            }

            // If we didn't insert (no registrations found), add after the function start
            if !inserted {
                let mut final_lines = Vec::new();
                let mut in_function = false;
                for line in new_lines {
                    final_lines.push(line);
                    if line.trim().starts_with("pub fn register_commands") {
                        in_function = true;
                    }
                    if in_function
                        && line.trim().starts_with("// Register all commands")
                        && !inserted
                    {
                        // Insert after the comment
                        final_lines.push(&reg_line);
                        inserted = true;
                    }
                }
                new_lines = final_lines;
            }

            let updated_content = new_lines.join("\n");
            fs::write(utils_rs_path, updated_content)?;
            crate::done!("✓ Added registration to utils::register_commands()");
        } else {
            crate::info!("✓ Registration already exists in utils::register_commands()");
        }
    }

    crate::info!("\nNext steps:");
    crate::info!("1. Edit the command file to implement your logic");
    crate::info!("2. Rebuild and run the bot");

    Ok(())
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn show_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    crate::info!("Discord Bot Rust v{}", VERSION);
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
