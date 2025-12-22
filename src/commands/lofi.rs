use crate::commands::Command;
use async_trait::async_trait;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::channel::Message;
use serenity::prelude::Context;
use songbird::input::ffmpeg;
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;

const LOFI_STREAM_URL: &str = "https://lofi4u.com/api/stream/live";

pub struct Lofi;

#[async_trait]
impl Command for Lofi {
    fn name(&self) -> &'static str {
        "lofi"
    }

    fn description(&self) -> &'static str {
        "Join your voice channel and stream lofi music 24/7"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("lofi")
            .description("Join your voice channel and stream lofi music 24/7");
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

        // Get the user's voice channel
        let user_id = interaction.user.id;
        let guild = match ctx.cache.guild(guild_id) {
            Some(guild) => guild,
            None => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Could not find guild information.")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        let voice_state = guild.voice_states.get(&user_id);
        let channel_id = match voice_state.and_then(|vs| vs.channel_id) {
            Some(id) => id,
            None => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("You need to be in a voice channel!")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::new().ephemeral(false),
                ),
            )
            .await?;

        // Get or create the songbird manager
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Songbird voice client not initialized"))?;

        // Join the voice channel
        let (handler_lock, success) = manager.join(guild_id, channel_id).await;

        match success {
            Ok(_) => {
                let mut handler = handler_lock.lock().await;

                // Create ffmpeg input for the stream
                let source = match ffmpeg(LOFI_STREAM_URL).await {
                    Ok(source) => source,
                    Err(e) => {
                        drop(handler);
                        manager.remove(guild_id).await.ok();
                        interaction
                            .create_followup(
                                &ctx.http,
                                serenity::builder::CreateInteractionResponseFollowup::new()
                                    .content(format!("Failed to create audio source: {}. Make sure FFmpeg is installed and accessible.", e))
                                    .ephemeral(true),
                            )
                            .await?;
                        return Ok(());
                    }
                };

                // Play the stream
                handler.play_source(source.into());

                interaction
                    .create_followup(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponseFollowup::new()
                            .embed(
                                CreateEmbed::new()
                                    .title("🎵 Lofi Stream Started")
                                    .description(format!(
                                        "Now streaming lofi music in <#{}>!\n\nThe bot will stay connected 24/7 until you use `/leave` or kick it.",
                                        channel_id
                                    ))
                                    .color(0x00ff00),
                            ),
                    )
                    .await?;
            }
            Err(e) => {
                interaction
                    .create_followup(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponseFollowup::new()
                            .content(format!("Failed to join voice channel: {}", e))
                            .ephemeral(true),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("lofi")
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

        // Get the user's voice channel
        let user_id = msg.author.id;
        let guild = match ctx.cache.guild(guild_id) {
            Some(guild) => guild,
            None => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Error")
                                .description("Could not find guild information.")
                                .color(0xff0000),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        let voice_state = guild.voice_states.get(&user_id);
        let channel_id = match voice_state.and_then(|vs| vs.channel_id) {
            Some(id) => id,
            None => {
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Error")
                                .description("You need to be in a voice channel!")
                                .color(0xff0000),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        // Get or create the songbird manager
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| anyhow::anyhow!("Songbird voice client not initialized"))?;

        // Join the voice channel
        let (handler_lock, success) = manager.join(guild_id, channel_id).await;

        match success {
            Ok(_) => {
                let mut handler = handler_lock.lock().await;

                // Create ffmpeg input for the stream
                let source = match ffmpeg(LOFI_STREAM_URL).await {
                    Ok(source) => source,
                    Err(e) => {
                        drop(handler);
                        manager.remove(guild_id).await.ok();
                        msg.channel_id
                            .send_message(
                                &ctx.http,
                                CreateMessage::new().embed(
                                    CreateEmbed::new()
                                        .title("Error")
                                        .description(format!("Failed to create audio source: {}. Make sure FFmpeg is installed and accessible.", e))
                                        .color(0xff0000),
                                ),
                            )
                            .await?;
                        return Ok(());
                    }
                };

                // Play the stream
                handler.play_source(source.into());

                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("🎵 Lofi Stream Started")
                                .description(format!(
                                    "Now streaming lofi music in <#{}>!\n\nThe bot will stay connected 24/7 until you use `/leave` or kick it.",
                                    channel_id
                                ))
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
                                .description(format!("Failed to join voice channel: {}", e))
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
    Arc::new(Lofi)
}

