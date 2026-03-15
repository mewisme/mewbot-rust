use crate::core::command::{CommandInfo, SubCommandInfo};
use crate::core::permissions::required_permission_message;
use crate::core::plugin_api::PluginApi;
use crate::core::Command;
use async_trait::async_trait;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;

fn build_command_embed(prefix: &str, info: &CommandInfo) -> CreateEmbed {
    let perm_str = match info.required_permission {
        Some(level) => required_permission_message(level).to_string(),
        None => "Any member".to_string(),
    };

    let slash_usage = format!("`/{}`", info.name);
    let prefix_usage = info
        .prefix
        .as_ref()
        .map(|p| format!("`{}{}`", prefix, p))
        .unwrap_or_else(|| "—".to_string());
    let aliases_str = if info.aliases.is_empty() {
        "—".to_string()
    } else {
        info.aliases
            .iter()
            .map(|a| format!("`{}{}`", prefix, a))
            .collect::<Vec<_>>()
            .join(", ")
    };

    CreateEmbed::new()
        .title(format!("Help: {}", info.name))
        .description(&info.description)
        .color(0x0099ff)
        .field("Slash", slash_usage, true)
        .field("Prefix", prefix_usage, true)
        .field("Aliases", aliases_str, false)
        .field("Cooldown", format!("{} seconds", info.cooldown_secs), true)
        .field("Permission", perm_str, true)
        .field("Version", &info.version, true)
}

fn build_command_with_subcommands_embed(prefix: &str, info: &CommandInfo) -> CreateEmbed {
    let mut embed = build_command_embed(prefix, info);
    if !info.subcommands.is_empty() {
        let sub_list: String = info
            .subcommands
            .iter()
            .map(|s| {
                let aliases_str = if s.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", s.aliases.join(", "))
                };
                format!("• **{}**{} — {}", s.name, aliases_str, s.description)
            })
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed
            .field("Subcommands", sub_list, false)
            .footer(CreateEmbedFooter::new(format!(
                "Use /help {} <subcommand> or {}{} {} <subcommand> for details",
                info.name,
                prefix,
                "help",
                info.name
            )));
    }
    embed
}

fn build_subcommand_embed(
    prefix: &str,
    cmd_name: &str,
    sub: &SubCommandInfo,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("Help: {} {}", cmd_name, sub.name))
        .description(sub.description)
        .color(0x0099ff)
        .field("Slash", format!("`/{} {}`", cmd_name, sub.name), true)
        .field("Prefix", format!("`{}{} {}`", prefix, cmd_name, sub.name), true);
    if !sub.aliases.is_empty() {
        let aliases_str = sub
            .aliases
            .iter()
            .map(|a| format!("`{}{} {}`", prefix, cmd_name, a))
            .collect::<Vec<_>>()
            .join(", ");
        embed = embed.field("Aliases", aliases_str, false);
    }
    embed
}

pub struct Help {
    api: PluginApi,
}

impl Help {
    pub fn new(api: PluginApi) -> Self {
        Self { api }
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
                    "The command (and optional subcommand) to get help for, e.g. wallet or wallet check",
                )
                .required(false),
            );
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {
        let command_arg = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "command")
            .and_then(|opt| opt.value.as_str());

        let commands = self.api.command_lister().list();
        let prefix = &self.api.config().command_prefix;

        let (command_name, sub_name) = parse_help_target(command_arg);

        if command_name.is_none() {
            let mut command_list = String::new();
            for info in &commands {
                command_list.push_str(&format!("• **{}** - {}\n", info.name, info.description));
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
            return Ok(());
        }

        let cmd_name = command_name.unwrap();
        let info = commands
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(cmd_name));

        match (info, sub_name) {
            (None, _) => {
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
            (Some(info), None) => {
                let embed = build_command_with_subcommands_embed(prefix, info);
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(embed),
                        ),
                    )
                    .await?;
            }
            (Some(info), Some(sub)) => {
                let sub_info = info.subcommands.iter().find(|s| {
                    s.name.eq_ignore_ascii_case(sub)
                        || s.aliases.iter().any(|a| a.eq_ignore_ascii_case(sub))
                });

                match sub_info {
                    Some(sub_info) => {
                        let embed = build_subcommand_embed(prefix, &info.name, sub_info);
                        interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().embed(embed),
                                ),
                            )
                            .await?;
                    }
                    None => {
                        interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .embed(
                                            CreateEmbed::new()
                                                .title("Subcommand Not Found")
                                                .description(format!(
                                                    "Subcommand `{}` not found for command `{}`.",
                                                    sub, info.name
                                                ))
                                                .color(0xff0000),
                                        )
                                        .ephemeral(true),
                                ),
                            )
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("help")
    }

    async fn run_prefix(&self, ctx: &Context, msg: &Message, args: &[&str]) -> anyhow::Result<()> {
        let commands = self.api.command_lister().list();
        let prefix = &self.api.config().command_prefix;

        let (command_name, sub_name) = if args.is_empty() {
            (None, None)
        } else {
            let first = args[0];
            let rest = args.get(1).copied();
            (Some(first), rest)
        };

        if command_name.is_none() {
            let mut command_list = String::new();
            for info in &commands {
                command_list.push_str(&format!("• **{}** - {}\n", info.name, info.description));
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
            msg.channel_id
                .send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed))
                .await?;
            return Ok(());
        }

        let cmd_name = command_name.unwrap();
        let info = commands
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(cmd_name));

        match (info, sub_name) {
            (None, _) => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        serenity::builder::CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Command Not Found")
                                .description(format!("Command `{}` not found.", cmd_name))
                                .color(0xff0000),
                        ),
                    )
                    .await?;
            }
            (Some(info), None) => {
                let embed = build_command_with_subcommands_embed(prefix, info);
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        serenity::builder::CreateMessage::new().embed(embed),
                    )
                    .await?;
            }
            (Some(info), Some(sub)) => {
                let sub_info = info.subcommands.iter().find(|s| {
                    s.name.eq_ignore_ascii_case(sub)
                        || s.aliases.iter().any(|a| a.eq_ignore_ascii_case(sub))
                });

                match sub_info {
                    Some(sub_info) => {
                        let embed = build_subcommand_embed(prefix, &info.name, sub_info);
                        msg.channel_id
                            .send_message(
                                &ctx.http,
                                serenity::builder::CreateMessage::new().embed(embed),
                            )
                            .await?;
                    }
                    None => {
                        msg.channel_id
                            .send_message(
                                &ctx.http,
                                serenity::builder::CreateMessage::new().embed(
                                    CreateEmbed::new()
                                        .title("Subcommand Not Found")
                                        .description(format!(
                                            "Subcommand `{}` not found for command `{}`.",
                                            sub, info.name
                                        ))
                                        .color(0xff0000),
                                ),
                            )
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["h", "commands"]
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

fn parse_help_target(arg: Option<&str>) -> (Option<&str>, Option<&str>) {
    let s = match arg {
        Some(x) => x.trim(),
        None => return (None, None),
    };
    if s.is_empty() {
        return (None, None);
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    let command_name = parts.first().copied();
    let sub_name = parts.get(1).copied();
    (command_name, sub_name)
}

pub fn create(api: PluginApi) -> Arc<Help> {
    Arc::new(Help::new(api))
}
