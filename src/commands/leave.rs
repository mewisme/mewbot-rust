use crate::commands::Command;
use async_trait::async_trait;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::channel::Message;
use serenity::prelude::Context;
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;

pub struct Leave;

#[async_trait]
impl Command for Leave {
    fn name(&self) -> &'static str {
        "leave"
    }

    fn description(&self) -> &'static str {
        "Disconnect the bot from the voice channel"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("leave")
            .description("Disconnect the bot from the voice channel");
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &serenity::model::application::CommandInteraction,
    ) -> anyhow::Result<()> {
        let guild_id = match interaction.guild_id {
            Some(id) => id,
            None => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("This command can only be used in a server!")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        // Get the songbird manager
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Songbird voice client not initialized"))?;

        // Check if bot is in a voice channel
        let has_handler = manager.get(guild_id).is_some();

        if !has_handler {
            interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("I'm not connected to any voice channel!")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }

        // Leave the voice channel
        match manager.remove(guild_id).await {
            Ok(_) => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(
                                    CreateEmbed::new()
                                        .title("👋 Left Voice Channel")
                                        .description("Successfully disconnected from the voice channel.")
                                        .color(0x00ff00),
                                )
                                .ephemeral(false),
                        ),
                    )
                    .await?;
            }
            Err(e) => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Failed to leave voice channel: {}", e))
                                .ephemeral(true),
                        ),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("leave")
    }

    async fn run_prefix(&self, ctx: &Context, msg: &Message, _args: &[&str]) -> anyhow::Result<()> {
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Error")
                                .description("This command can only be used in a server!")
                                .color(0xff0000),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        // Get the songbird manager
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Songbird voice client not initialized"))?;

        // Check if bot is in a voice channel
        let has_handler = manager.get(guild_id).is_some();

        if !has_handler {
            msg.channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .title("Error")
                            .description("I'm not connected to any voice channel!")
                            .color(0xff0000),
                    ),
                )
                .await?;
            return Ok(());
        }

        // Leave the voice channel
        match manager.remove(guild_id).await {
            Ok(_) => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("👋 Left Voice Channel")
                                .description("Successfully disconnected from the voice channel.")
                                .color(0x00ff00),
                        ),
                    )
                    .await?;
            }
            Err(e) => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Error")
                                .description(format!("Failed to leave voice channel: {}", e))
                                .color(0xff0000),
                        ),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(3)
    }
}

pub fn create() -> Arc<dyn Command> {
    Arc::new(Leave)
}

