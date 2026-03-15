use crate::commands::Command;
use crate::config::Config;
use crate::permissions::required_permission_message;
use crate::registry::Registry;
use async_trait::async_trait;
use serenity::all::CreateMessage;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

fn build_command_embed(prefix: &str, cmd: &dyn Command) -> CreateEmbed {
    let cooldown = cmd.cooldown_duration().as_secs();
    let aliases = cmd.aliases();
    let perm_str = match cmd.required_permission_level() {
        Some(level) => required_permission_message(level).to_string(),
        None => "Any member".to_string(),
    };

    let slash_usage = format!("`/{}`", cmd.name());
    let prefix_usage = cmd
        .prefix()
        .map(|p| format!("`{}{}`", prefix, p))
        .unwrap_or_else(|| "—".to_string());
    let aliases_str = if aliases.is_empty() {
        "—".to_string()
    } else {
        aliases
            .iter()
            .map(|a| format!("`{}{}`", prefix, a))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let embed = CreateEmbed::new()
        .title(format!("Help: {}", cmd.name()))
        .description(cmd.description())
        .color(0x0099ff)
        .field("Slash", slash_usage, true)
        .field("Prefix", prefix_usage, true)
        .field("Aliases", aliases_str, false)
        .field("Cooldown", format!("{} seconds", cooldown), true)
        .field("Permission", perm_str, true);

    embed
}

pub struct Help {
    registry: Arc<Mutex<Registry>>,
    config: Config,
}

impl Help {
    pub fn new(registry: Arc<Mutex<Registry>>, config: Config) -> Self {
        Self { registry, config }
    }
}

#[async_trait]
impl Command for Help {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show help information about commands"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("help")
            .description("Show help information about commands")
            .add_option(
                serenity::builder::CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "command",
                    "The command to get help for",
                )
                .required(false),
            );
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {
        let registry = self.registry.lock().await;
        let commands = registry.all_commands();
        drop(registry);

        let command_name = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "command")
            .and_then(|opt| opt.value.as_str());

        if let Some(cmd_name) = command_name {
            let command = commands
                .iter()
                .find(|c| c.name().eq_ignore_ascii_case(cmd_name));

            if let Some(cmd) = command {
                let prefix = &self.config.command_prefix;
                let embed = build_command_embed(prefix, cmd.as_ref());
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(embed),
                        ),
                    )
                    .await?;
            } else {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(
                                    CreateEmbed::new()
                                        .title("Command Not Found")
                                        .description(format!("Command `{}` not found.", cmd_name))
                                        .color(0xff0000),
                                )
                                .ephemeral(true),
                        ),
                    )
                    .await?;
            }
        } else {
            let prefix = &self.config.command_prefix;
            let mut command_list = String::new();

            for cmd in &commands {
                command_list.push_str(&format!("• **{}** - {}\n", cmd.name(), cmd.description()));
            }

            interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("Available Commands")
                                .description(&command_list)
                                .footer(CreateEmbedFooter::new(format!(
                                    "Use /help <command> or {}{} <command> for detailed info",
                                    prefix,
                                    self.name()
                                )))
                                .color(0x0099ff),
                        ),
                    ),
                )
                .await?;
        }

        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("help")
    }

    async fn run_prefix(&self, ctx: &Context, msg: &Message, args: &[&str]) -> anyhow::Result<()> {
        let registry = self.registry.lock().await;
        let commands = registry.all_commands();
        drop(registry);

        let prefix = &self.config.command_prefix;

        if let Some(cmd_name) = args.first() {
            let command = commands
                .iter()
                .find(|c| c.name().eq_ignore_ascii_case(cmd_name));

            if let Some(cmd) = command {
                let embed = build_command_embed(prefix, cmd.as_ref());
                let response = CreateMessage::new().embed(embed);
                msg.channel_id.send_message(&ctx.http, response).await?;
            } else {
                let response = CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title("Command Not Found")
                        .description(format!("Command `{}` not found.", cmd_name))
                        .color(0xff0000),
                );

                msg.channel_id.send_message(&ctx.http, response).await?;
            }
        } else {
            let mut command_list = String::new();

            for cmd in &commands {
                command_list.push_str(&format!("• **{}** - {}\n", cmd.name(), cmd.description()));
            }

            let embed = CreateEmbed::new()
                .title("Available Commands")
                .description(command_list)
                .footer(CreateEmbedFooter::new(format!(
                    "Use {}{} <command> for detailed info",
                    prefix,
                    self.name()
                )))
                .color(0x0099ff);

            let msg_builder = CreateMessage::new().embed(embed);

            msg.channel_id.send_message(&ctx.http, msg_builder).await?;
        }

        Ok(())
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["h", "commands"]
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(2)
    }
}

pub fn create(registry: Arc<Mutex<Registry>>, config: Config) -> Arc<Help> {
    Arc::new(Help::new(registry, config))
}
