use crate::context::BotContext;
use crate::permissions::{get_permission_level, has_permission, required_permission_message};
use crate::utils;
use serenity::builder::{
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::Interaction;
use serenity::model::permissions::Permissions;
use serenity::prelude::Context;

pub async fn interaction(ctx: Context, interaction: Interaction, bot_context: &BotContext) {
    if let Interaction::Command(cmd_interaction) = interaction {
        let command_name = &cmd_interaction.data.name;

        let registry = bot_context.registry.lock().await;
        let command = match registry.get_slash_command(command_name) {
            Some(cmd) => cmd,
            None => {
                crate::error!("Unknown slash command: {}", command_name);
                return;
            }
        };
        drop(registry);

        if let Some(required) = command.required_permission_level() {
            let guild_id = match cmd_interaction.guild_id {
                Some(id) => id,
                None => {
                    let _ = cmd_interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("This command can only be used in a server.")
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                    return;
                }
            };
            let permission_data = match ctx.cache.guild(guild_id) {
                Some(guild) => {
                    let guild_owner_id = guild.owner_id;
                    let channel_id = cmd_interaction.channel_id;
                    let has_admin = cmd_interaction
                        .member
                        .as_ref()
                        .and_then(|m| {
                            guild
                                .channels
                                .get(&channel_id)
                                .map(|ch| guild.user_permissions_in(ch, m).contains(Permissions::ADMINISTRATOR))
                        })
                        .unwrap_or(false);
                    Some((guild_owner_id, has_admin))
                }
                None => None,
            };
            let (guild_owner_id, has_administrator) = match permission_data {
                Some((o, h)) => (o, h),
                None => {
                    let _ = cmd_interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("Could not load server information.")
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                    return;
                }
            };
            let user_level =
                get_permission_level(guild_owner_id, cmd_interaction.user.id, has_administrator);
            if !has_permission(user_level, required) {
                let required_str = required_permission_message(required);
                let _ = cmd_interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(
                                    CreateEmbed::new()
                                        .title("Permission Denied")
                                        .description(format!("This command requires **{}** permission.", required_str))
                                        .color(0xff0000),
                                )
                                .ephemeral(true),
                    ),
                    )
                    .await;
                return;
            }
        }

        let user_id = cmd_interaction.user.id.get();
        let cooldown_duration = command.cooldown_duration();

        if let Some(remaining) = bot_context
            .get_cooldown_remaining(user_id, command.name(), cooldown_duration)
            .await
        {
            let remaining_str = utils::format_duration(remaining);
            let error_msg = format!("You are on cooldown for {} more seconds", remaining_str);

            use serenity::builder::CreateInteractionResponse;
            use serenity::builder::CreateInteractionResponseMessage;

            if let Err(e) = cmd_interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(
                                serenity::builder::CreateEmbed::new()
                                    .title("Cooldown")
                                    .description(&error_msg)
                                    .color(0xff0000),
                            )
                            .ephemeral(true),
                    ),
                )
                .await
            {
                crate::error!("Failed to send cooldown message: {}", e);
            }
            return;
        }

        match command.run_slash(&ctx, &cmd_interaction).await {
            Ok(_) => {
                bot_context.set_cooldown(user_id, command.name()).await;
            }
            Err(e) => {
                let error_msg = utils::format_error(&e);

                use serenity::builder::CreateInteractionResponse;
                use serenity::builder::CreateInteractionResponseMessage;

                if let Err(err) = cmd_interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(
                                    serenity::builder::CreateEmbed::new()
                                        .title("Error")
                                        .description(&error_msg)
                                        .color(0xff0000),
                                )
                                .ephemeral(true),
                        ),
                    )
                    .await
                {
                    crate::error!("Failed to send error message: {}", err);
                }
            }
        }
    }
}
