use crate::context::BotContext;
use crate::utils;
use serenity::model::application::Interaction;
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
